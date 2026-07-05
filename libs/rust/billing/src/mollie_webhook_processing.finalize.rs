use serde_json::json;

use crate::mollie_webhook_processing::{
    MollieInitialSubscriptionRequest, MollieWebhookProcessingResult,
};
use crate::provider::{ProviderCode, ProviderSubscription};

pub async fn finalize_mollie_initial_subscription_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &MollieInitialSubscriptionRequest,
    subscription: &ProviderSubscription,
) -> MollieWebhookProcessingResult<()> {
    let activated = crate::db::activate_mollie_subscription_after_initial_payment_tx(
        tx,
        request.workspace_id,
        request.plan_id,
        &request.provider_customer_id,
        &subscription.provider_subscription_id,
        request.current_period_start,
        request.current_period_end,
    )
    .await?;

    if activated {
        crate::db::upsert_provider_subscription_tx(
            tx,
            request.workspace_id,
            ProviderCode::Mollie,
            &request.provider_customer_id,
            &subscription.provider_subscription_id,
            &subscription.status,
            Some(request.current_period_start),
            Some(request.current_period_end),
            true,
            json!({
                "provider_payment_id": request.provider_payment_id,
                "created_from": "initial_payment",
            }),
        )
        .await?;
        crate::db::project_workspace_plan_tx(tx, request.workspace_id, request.plan_id).await?;
        crate::entitlements_db::persist_current_workspace_entitlements_tx(
            tx,
            request.workspace_id,
            None,
            "mollie_subscription_created",
            json!({
                "provider": ProviderCode::Mollie.as_str(),
                "provider_payment_id": request.provider_payment_id,
                "provider_subscription_id": subscription.provider_subscription_id,
            }),
        )
        .await?;
    }

    crate::db::insert_billing_audit_tx(
        tx,
        request.workspace_id,
        "billing.mollie_subscription_created",
        json!({
            "provider": ProviderCode::Mollie.as_str(),
            "provider_payment_id": request.provider_payment_id,
            "provider_subscription_id": subscription.provider_subscription_id,
            "status": subscription.status,
        }),
    )
    .await?;
    Ok(())
}
