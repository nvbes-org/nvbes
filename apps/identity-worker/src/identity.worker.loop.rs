use std::{
    error::Error,
    time::{Duration, Instant},
};

use nvbes_observability::{
    WorkerMonitorSchedule, WorkerOperationContext, capture_worker_heartbeat,
    capture_worker_operation_error, worker_monitor_slug,
};
use tokio::time::{Duration as TokioDuration, sleep};

use crate::app::AppState;
use nvbes_product_identity::email::{jobs::EmailSendPayload, templates::html_escape};

use super::job_processor::{WORKER_QUEUES, claim_available_job, process_claimed_job};

const ACCESS_REVIEW_SCHEDULE_INTERVAL: Duration = Duration::from_secs(900);
const ACCESS_REVIEW_REMINDER_INTERVAL: Duration = Duration::from_secs(3600);
const WORKER_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);
const TRANSIENT_INFRA_ERROR_SLEEP: TokioDuration = TokioDuration::from_secs(5);
const WORKER_HEARTBEAT_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 5,
    checkin_margin_minutes: 2,
    max_runtime_minutes: 2,
};

pub async fn run_loop_until_shutdown<S>(state: AppState, shutdown: S) -> anyhow::Result<()>
where
    S: std::future::Future<Output = ()> + Send,
{
    let observability = state.observability.clone();
    let mut access_review_schedule_last_run = Instant::now() - ACCESS_REVIEW_SCHEDULE_INTERVAL;
    let mut access_review_reminder_last_run = Instant::now() - ACCESS_REVIEW_REMINDER_INTERVAL;
    let mut housekeeping_last_run = Instant::now() - Duration::from_secs(3600);
    let mut worker_heartbeat_last_run = Instant::now() - WORKER_HEARTBEAT_INTERVAL;
    let mut queue_metrics_last_run = Instant::now() - super::queue_metrics::REFRESH_INTERVAL;
    tokio::pin!(shutdown);
    loop {
        capture_worker_heartbeat_if_due(&state, &mut worker_heartbeat_last_run);
        if let Err(error) = super::queue_metrics::refresh_if_due(
            &state,
            &WORKER_QUEUES,
            &mut queue_metrics_last_run,
        )
        .await
        {
            tracing::warn!(%error, "identity worker queue metrics refresh failed");
        }
        if let Err(error) = super::account_projection::process_next(&state).await {
            capture_loop_error(&state, "account_registration_projection", error.as_ref());
            tracing::warn!(%error, "Account registration projection loop failed");
            sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
            continue;
        }
        if let Err(error) =
            run_access_review_schedules_if_due(&state, &mut access_review_schedule_last_run).await
        {
            capture_loop_error(&state, "access_review_schedules", error.as_ref());
            return Err(error);
        }
        if let Err(error) =
            run_access_review_reminders_if_due(&state, &mut access_review_reminder_last_run).await
        {
            capture_loop_error(&state, "access_review_reminders", error.as_ref());
            return Err(error);
        }
        if state.inline_housekeeping_enabled {
            tokio::select! {
                _ = &mut shutdown => return Ok(()),
                result = super::housekeeping::run_if_due(&state, &mut housekeeping_last_run) => {
                    if let Err(error) = result {
                        capture_loop_error(&state, "housekeeping", error.as_ref());
                        tracing::warn!(%error, "identity worker housekeeping failed; retrying after backoff");
                        sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
                        continue;
                    }
                }
            }
        }
        let claimed_job = tokio::select! {
            biased;
            result = claim_available_job(&state, &observability) => result,
            _ = &mut shutdown => return Ok(()),
        };
        let result = match claimed_job {
            Ok(Some(job)) => process_claimed_job(&state, &observability, job)
                .await
                .map(|()| true),
            Ok(None) => Ok(false),
            Err(error) => Err(error),
        };
        let ran = match result {
            Ok(ran) => ran,
            Err(error) => {
                capture_loop_error(&state, "worker_loop", error.as_ref());
                tracing::warn!(%error, "identity worker loop failed; retrying after backoff");
                sleep(TRANSIENT_INFRA_ERROR_SLEEP).await;
                continue;
            }
        };
        let sleep_for = if ran {
            TokioDuration::from_secs(1)
        } else {
            TokioDuration::from_secs(5)
        };
        tokio::select! {
            _ = &mut shutdown => return Ok(()),
            _ = sleep(sleep_for) => {},
        }
    }
}

async fn run_access_review_reminders_if_due(
    state: &AppState,
    last_run: &mut Instant,
) -> anyhow::Result<()> {
    if last_run.elapsed() < ACCESS_REVIEW_REMINDER_INTERVAL {
        return Ok(());
    }
    let Some(claim) = super::enterprise_grpc::claim_access_review_reminder_candidates(
        state.enterprise_grpc.as_ref(),
    )
    .await?
    else {
        *last_run = Instant::now();
        return Ok(());
    };
    for candidate in &claim.candidates {
        let payload = reminder_email_payload(&state.config, candidate);
        nvbes_product_identity::email::jobs::enqueue_email_job_tx(
            &state.db,
            &state.redis,
            payload,
            &format!(
                "access-review-reminder:{}:{}:{}",
                candidate.campaign_id, candidate.recipient_principal_id, candidate.reminder_kind
            ),
        )
        .await
        .map_err(|error| anyhow::anyhow!("{}: {}", error.code, error.message))?;
    }
    if !claim.candidates.is_empty() {
        tracing::info!(
            reminders_enqueued = claim.candidates.len(),
            "enqueued access review reminders"
        );
    }
    *last_run = Instant::now();
    Ok(())
}

fn reminder_email_payload(
    config: &nvbes_core::config::AppConfig,
    candidate: &crate::grpc_pb::nvbes::enterprise::v1::AccessReviewReminderCandidate,
) -> EmailSendPayload {
    let link = format!(
        "{}/access-reviews",
        config.web_base_url.trim_end_matches('/')
    );
    let subject = if candidate.reminder_kind == "overdue" {
        format!("Access review overdue: {}", candidate.campaign_name)
    } else {
        format!("Access review due soon: {}", candidate.campaign_name)
    };
    let text_body = format!(
        "{}\n\nTenant: {}\nPending items: {}\nDue: {}\n\nOpen access reviews: {}",
        subject, candidate.tenant_name, candidate.pending_items, candidate.due_at, link
    );
    let html_body = format!(
        "<p>{}</p><p><strong>Tenant:</strong> {}<br><strong>Pending items:</strong> {}<br><strong>Due:</strong> {}</p><p><a href=\"{}\">Open access reviews</a></p>",
        html_escape(&subject),
        html_escape(&candidate.tenant_name),
        candidate.pending_items,
        html_escape(&candidate.due_at),
        html_escape(&link)
    );

    EmailSendPayload {
        to_email: candidate.recipient_email.clone(),
        to_name: Some(candidate.recipient_name.clone()),
        subject,
        html_body,
        text_body: Some(text_body),
        business_type: "access_review_reminder".to_string(),
    }
}

async fn run_access_review_schedules_if_due(
    state: &AppState,
    last_run: &mut Instant,
) -> anyhow::Result<()> {
    if last_run.elapsed() < ACCESS_REVIEW_SCHEDULE_INTERVAL {
        return Ok(());
    }
    let Some(run) = super::enterprise_grpc::materialize_due_access_review_schedules(
        state.enterprise_grpc.as_ref(),
    )
    .await?
    else {
        *last_run = Instant::now();
        return Ok(());
    };
    if run.campaigns_created > 0 || run.empty_schedules > 0 {
        tracing::info!(
            campaigns_created = run.campaigns_created,
            empty_schedules = run.empty_schedules,
            "materialized due access review schedules"
        );
    }
    *last_run = Instant::now();
    Ok(())
}

fn capture_loop_error(
    state: &AppState,
    operation: &'static str,
    error: &(dyn Error + Send + Sync + 'static),
) {
    capture_worker_operation_error(
        error,
        &WorkerOperationContext {
            app_name: "identity-worker",
            environment: &state.config.environment,
            operation,
        },
    );
}

fn capture_worker_heartbeat_if_due(state: &AppState, last_run: &mut Instant) {
    if last_run.elapsed() < WORKER_HEARTBEAT_INTERVAL {
        return;
    }

    capture_worker_heartbeat(
        &state.config.environment,
        &worker_monitor_slug("identity-worker", "loop-heartbeat"),
        WORKER_HEARTBEAT_SCHEDULE,
    );
    state
        .observability
        .record_worker_heartbeat("identity-worker");
    *last_run = Instant::now();
}
