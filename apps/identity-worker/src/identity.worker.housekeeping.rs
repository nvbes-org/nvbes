use std::time::{Duration, Instant};

use nvbes_observability::{
    WorkerMonitorSchedule, start_worker_monitor_check_in, worker_monitor_slug,
};

use crate::app::AppState;

const HOUSEKEEPING_INTERVAL: Duration = Duration::from_secs(3600);
const HOUSEKEEPING_MONITOR_SCHEDULE: WorkerMonitorSchedule = WorkerMonitorSchedule {
    interval_minutes: 60,
    checkin_margin_minutes: 15,
    max_runtime_minutes: 30,
};

pub async fn run_if_due(state: &AppState, last_run: &mut Instant) -> anyhow::Result<()> {
    if last_run.elapsed() < HOUSEKEEPING_INTERVAL {
        return Ok(());
    }

    let check_in = start_worker_monitor_check_in(
        &state.config.environment,
        &worker_monitor_slug(
            "identity-worker",
            "housekeeping-expired-unverified-accounts",
        ),
        HOUSEKEEPING_MONITOR_SCHEDULE,
    );

    let result =
        nvbes_identity_api::domains::auth::email_verification::cleanup_expired_unverified_accounts(
            &state.db,
            state.config.auth_unverified_account_ttl_days,
        )
        .await
        .map_err(|error| anyhow::anyhow!(error.message));

    let deleted = match result {
        Ok(deleted) => {
            check_in.finish_ok();
            deleted
        }
        Err(error) => {
            check_in.finish_error();
            return Err(error);
        }
    };

    if deleted > 0 {
        tracing::info!(deleted, "Expired unverified accounts cleaned up");
    }

    *last_run = Instant::now();
    Ok(())
}
