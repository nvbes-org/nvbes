use anyhow::Context;
use nvbes_billing::provider::{ProviderCode, ProviderPayment};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::warn;
use uuid::Uuid;

use crate::worker::{
    BillingWorkerState, email::enqueue_billing_email_for_provider_payment,
    workspace_updates::publish_workspace_billing_updates,
};

#[derive(Debug, Deserialize)]
struct MollieWebhookJobPayload {
    provider_event_id: String,
}

pub async fn process_mollie_webhook_job(
    state: &BillingWorkerState,
    job: &nvbes_redis::worker_queue::QueuedJob,
) -> anyhow::Result<Value> {
    let job_payload: MollieWebhookJobPayload = serde_json::from_value(job.payload.clone())
        .context("Invalid Mollie webhook job payload")?;
    let result = process_mollie_webhook_event(state, &job_payload.provider_event_id).await;
    if let Err(error) = &result {
        mark_mollie_event_failed(state, &job_payload.provider_event_id, error).await;
    }
    result
}

async fn process_mollie_webhook_event(
    state: &BillingWorkerState,
    provider_event_id: &str,
) -> anyhow::Result<Value> {
    let payment = crate::worker::mollie::fetch_mollie_payment(&state.config, provider_event_id)
        .await
        .context("Mollie payment fetch failed")?;
    let mut tx = state.db.begin().await?;
    let outcome = nvbes_billing::mollie_webhook_processing::process_mollie_payment_update_tx(
        &mut tx,
        &state.config.billing_api_base_url(),
        &payment,
    )
    .await
    .context("Mollie webhook processing failed")?;
    tx.commit().await?;

    let workspace_id = outcome.workspace_id;
    let activates_subscription = outcome.initial_subscription.is_some();
    if let Some(request) = outcome.initial_subscription {
        let subscription = crate::worker::mollie::create_mollie_subscription(
            &state.config,
            &request.subscription_input,
        )
        .await
        .context("Mollie subscription creation failed")?;
        let mut tx = state.db.begin().await?;
        nvbes_billing::mollie_webhook_processing::finalize_mollie_initial_subscription_tx(
            &mut tx,
            &request,
            &subscription,
        )
        .await
        .context("Mollie initial subscription finalization failed")?;
        tx.commit().await?;
    }

    let tenant_id = match workspace_id {
        Some(workspace_id) => tenant_id_for_workspace(&state.db, workspace_id).await?,
        None => None,
    };
    nvbes_billing::db::mark_provider_event_processed(
        &state.db,
        ProviderCode::Mollie,
        provider_event_id,
        tenant_id,
    )
    .await?;

    let plan_code = match workspace_id {
        Some(workspace_id) => {
            let plan_code = publish_workspace_billing_updates(state, workspace_id).await?;
            capture_mollie_webhook_analytics(
                state,
                workspace_id,
                &payment,
                plan_code.as_deref(),
                activates_subscription,
            );
            enqueue_billing_email_for_provider_payment(
                &state.db,
                &state.redis,
                workspace_id,
                provider_event_id,
                &payment,
            )
            .await
            .context("Billing email enqueue failed")?;
            plan_code
        }
        None => None,
    };

    Ok(json!({
        "status": "processed",
        "provider": ProviderCode::Mollie.as_str(),
        "provider_event_id": provider_event_id,
        "workspace_id": workspace_id,
        "plan_code": plan_code,
    }))
}

async fn tenant_id_for_workspace(
    db_pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>("SELECT tenant_id FROM workspaces WHERE id = $1")
        .bind(workspace_id)
        .fetch_optional(db_pool)
        .await
}

fn capture_mollie_webhook_analytics(
    state: &BillingWorkerState,
    workspace_id: Uuid,
    payment: &ProviderPayment,
    plan_code: Option<&str>,
    activates_subscription: bool,
) {
    if activates_subscription {
        let mut event = nvbes_product_analytics::ProductAnalyticsEvent::workspace(
            "billing.subscription_activated",
            workspace_id,
        )
        .property("provider", ProviderCode::Mollie.as_str())
        .property("status", "active");
        if let Some(plan_code) = plan_code {
            event = event.property("plan_code", plan_code.to_string());
        }
        state.product_analytics.capture(event);
    } else if payment.status == "failed" {
        state.product_analytics.capture(
            nvbes_product_analytics::ProductAnalyticsEvent::workspace(
                "billing.payment_failed",
                workspace_id,
            )
            .property("provider", ProviderCode::Mollie.as_str())
            .property("status", "past_due"),
        );
    }
}

async fn mark_mollie_event_failed(
    state: &BillingWorkerState,
    provider_event_id: &str,
    error: &anyhow::Error,
) {
    let (code, message) = mollie_failure_details(error);
    if let Err(mark_error) = nvbes_billing::db::mark_provider_event_failed(
        &state.db,
        ProviderCode::Mollie,
        provider_event_id,
        code,
        &message,
    )
    .await
    {
        warn!(
            provider_event_id,
            error = %mark_error,
            "Failed to mark Mollie provider event as failed"
        );
    }
}

fn mollie_failure_details(error: &anyhow::Error) -> (&'static str, String) {
    if let Some(error) = error.chain().find_map(|cause| {
        cause
            .downcast_ref::<nvbes_billing::mollie_webhook_processing::MollieWebhookProcessingError>(
            )
    }) {
        return (error.code(), error.message().to_string());
    }
    if let Some(error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<nvbes_billing::mollie::MollieProviderError>())
    {
        return (error.code(), error.message().to_string());
    }
    ("mollie_webhook_processing_failed", error.to_string())
}
