use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[path = "mollie_webhook_processing.finalize.rs"]
mod finalize;

pub use finalize::finalize_mollie_initial_subscription_tx;

use crate::provider::{ProviderCode, ProviderPayment, ProviderSubscriptionInput};

pub type MollieWebhookProcessingResult<T> = Result<T, MollieWebhookProcessingError>;

#[derive(Debug, Error)]
pub enum MollieWebhookProcessingError {
    #[error("{code}: {message}")]
    BadRequest {
        code: &'static str,
        message: &'static str,
    },
    #[error("{code}: {message}")]
    NotFound {
        code: &'static str,
        message: &'static str,
    },
    #[error("mollie webhook processing database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl MollieWebhookProcessingError {
    pub fn bad_request(code: &'static str, message: &'static str) -> Self {
        Self::BadRequest { code, message }
    }

    pub fn not_found(code: &'static str, message: &'static str) -> Self {
        Self::NotFound { code, message }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest { code, .. } | Self::NotFound { code, .. } => code,
            Self::Database(_) => "mollie_webhook_processing_failed",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::BadRequest { message, .. } | Self::NotFound { message, .. } => message,
            Self::Database(_) => "Mollie webhook processing failed.",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MollieInitialSubscriptionRequest {
    pub workspace_id: Uuid,
    pub plan_id: Uuid,
    pub provider_customer_id: String,
    pub provider_payment_id: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub subscription_input: ProviderSubscriptionInput,
}

#[derive(Debug, Clone)]
pub struct MolliePaymentProcessingOutcome {
    pub workspace_id: Option<Uuid>,
    pub initial_subscription: Option<MollieInitialSubscriptionRequest>,
}

pub async fn process_mollie_payment_update_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    api_base_url: &str,
    payment: &ProviderPayment,
) -> MollieWebhookProcessingResult<MolliePaymentProcessingOutcome> {
    let Some(provider_customer_id) = payment.provider_customer_id.as_deref() else {
        return Ok(MolliePaymentProcessingOutcome {
            workspace_id: None,
            initial_subscription: None,
        });
    };
    let workspace_id = crate::db::workspace_id_for_provider_customer_code_tx(
        tx,
        ProviderCode::Mollie,
        provider_customer_id,
    )
    .await?
    .ok_or_else(|| {
        MollieWebhookProcessingError::bad_request(
            "unknown_provider_customer",
            "Provider customer is not mapped to a workspace.",
        )
    })?;

    persist_payment_method_and_signal(tx, workspace_id, provider_customer_id, payment).await?;

    if payment.status != "captured" {
        crate::db::insert_billing_audit_tx(
            tx,
            workspace_id,
            "billing.mollie_payment_updated",
            json!({
                "provider": ProviderCode::Mollie.as_str(),
                "provider_payment_id": payment.provider_payment_id,
                "status": payment.status,
            }),
        )
        .await?;
        return Ok(MolliePaymentProcessingOutcome {
            workspace_id: Some(workspace_id),
            initial_subscription: None,
        });
    }

    if let Some(provider_subscription_id) = payment.provider_subscription_id.as_deref() {
        mark_mollie_subscription_payment_captured_tx(
            tx,
            workspace_id,
            provider_subscription_id,
            payment,
        )
        .await?;
        return Ok(MolliePaymentProcessingOutcome {
            workspace_id: Some(workspace_id),
            initial_subscription: None,
        });
    }

    let initial_subscription = mollie_initial_subscription_request_tx(
        tx,
        api_base_url,
        workspace_id,
        provider_customer_id,
        payment,
    )
    .await?;
    Ok(MolliePaymentProcessingOutcome {
        workspace_id: Some(workspace_id),
        initial_subscription,
    })
}

async fn persist_payment_method_and_signal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider_customer_id: &str,
    payment: &ProviderPayment,
) -> MollieWebhookProcessingResult<()> {
    if let Some(payment_method) = &payment.payment_method {
        crate::db::upsert_provider_payment_method_tx(
            tx,
            ProviderCode::Mollie,
            provider_customer_id,
            payment_method,
        )
        .await?;
    }
    if let Some(signal) = crate::mollie_payment_signal(payment) {
        crate::db::insert_billing_fraud_psp_signal_tx(tx, workspace_id, &signal).await?;
    }
    Ok(())
}

async fn mollie_initial_subscription_request_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    api_base_url: &str,
    workspace_id: Uuid,
    provider_customer_id: &str,
    payment: &ProviderPayment,
) -> MollieWebhookProcessingResult<Option<MollieInitialSubscriptionRequest>> {
    if current_provider_subscription_id_tx(tx, workspace_id)
        .await?
        .is_some()
    {
        return Ok(None);
    }

    let plan_code = payment.plan_code.as_deref().ok_or_else(|| {
        MollieWebhookProcessingError::bad_request(
            "mollie_payment_missing_plan",
            "Mollie initial payment is missing plan metadata.",
        )
    })?;
    let plan_id = crate::db::plan_id_by_code_tx(tx, plan_code)
        .await?
        .ok_or_else(|| {
            MollieWebhookProcessingError::not_found("plan_not_found", "Plan not found.")
        })?;
    let now = Utc::now();
    let period_end = now + Duration::days(30);

    Ok(Some(MollieInitialSubscriptionRequest {
        workspace_id,
        plan_id,
        provider_customer_id: provider_customer_id.to_string(),
        provider_payment_id: payment.provider_payment_id.clone(),
        current_period_start: now,
        current_period_end: period_end,
        subscription_input: ProviderSubscriptionInput {
            provider_customer_id: provider_customer_id.to_string(),
            amount_minor: payment.amount_minor,
            currency: payment.currency.clone(),
            interval: "1 month".to_string(),
            description: format!("nvbes {workspace_id} {plan_code} monthly"),
            start_date: Some(period_end.date_naive().to_string()),
            webhook_url: Some(format!(
                "{}/webhooks/mollie",
                api_base_url.trim_end_matches('/')
            )),
        },
    }))
}

async fn mark_mollie_subscription_payment_captured_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    provider_subscription_id: &str,
    payment: &ProviderPayment,
) -> MollieWebhookProcessingResult<()> {
    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'active',
            updated_at = NOW()
        WHERE workspace_id = $1
          AND billing_provider = 'mollie'
          AND billing_subscription_id = $2
          AND status IN ('past_due', 'suspended', 'incomplete', 'active')
        "#,
    )
    .bind(workspace_id)
    .bind(provider_subscription_id)
    .execute(tx.as_mut())
    .await?;

    if let Some(provider_customer_id) = payment.provider_customer_id.as_deref() {
        crate::db::upsert_provider_subscription_tx(
            tx,
            workspace_id,
            ProviderCode::Mollie,
            provider_customer_id,
            provider_subscription_id,
            "active",
            None,
            None,
            true,
            json!({
                "provider_payment_id": payment.provider_payment_id,
                "updated_from": "recurring_payment",
            }),
        )
        .await?;
    }

    crate::db::insert_billing_audit_tx(
        tx,
        workspace_id,
        "billing.mollie_payment_succeeded",
        json!({
            "provider": ProviderCode::Mollie.as_str(),
            "provider_payment_id": payment.provider_payment_id,
            "provider_subscription_id": provider_subscription_id,
        }),
    )
    .await?;
    Ok(())
}

async fn current_provider_subscription_id_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, Option<String>>(
        r#"
        SELECT billing_subscription_id
        FROM subscriptions
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(tx.as_mut())
    .await
    .map(Option::flatten)
}
