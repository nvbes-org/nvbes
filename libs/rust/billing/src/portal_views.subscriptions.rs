use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::provider::ProviderCode;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BillingPortalSubscriptionProviderView {
    #[schema(value_type = ProviderCode)]
    pub provider: String,
    pub status: String,
    pub primary: bool,
    pub fallback_eligible: bool,
    pub activated_at: Option<DateTime<Utc>>,
    pub deactivated_at: Option<DateTime<Utc>>,
}

pub async fn fetch_portal_subscription_providers(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<BillingPortalSubscriptionProviderView>, sqlx::Error> {
    let records = sqlx::query_as::<_, BillingPortalSubscriptionProviderRecord>(
        r#"
        WITH canonical_providers AS (
          SELECT bps.provider::text AS provider,
                 bps.status,
                 bps.primary_for_subscription AS primary,
                 bps.fallback_eligible,
                 bps.activated_at,
                 bps.deactivated_at
          FROM billing_provider_subscriptions bps
          JOIN billing_subscriptions bs ON bs.id = bps.subscription_id
          WHERE bs.workspace_id = $1
        ),
        legacy_provider AS (
          SELECT s.billing_provider::text AS provider,
                 s.status::text AS status,
                 TRUE AS primary,
                 FALSE AS fallback_eligible,
                 s.current_period_start AS activated_at,
                 CASE
                   WHEN s.status::text IN ('canceled', 'cancelled')
                   THEN s.current_period_end
                   ELSE NULL
                 END AS deactivated_at
          FROM subscriptions s
          WHERE s.workspace_id = $1
            AND s.billing_subscription_id IS NOT NULL
            AND NOT EXISTS (SELECT 1 FROM canonical_providers)
        )
        SELECT provider,
               status,
               primary,
               fallback_eligible,
               activated_at,
               deactivated_at
        FROM canonical_providers
        UNION ALL
        SELECT provider,
               status,
               primary,
               fallback_eligible,
               activated_at,
               deactivated_at
        FROM legacy_provider
        ORDER BY primary DESC,
                 fallback_eligible DESC,
                 activated_at DESC NULLS LAST,
                 provider
        "#,
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await?;

    Ok(records.into_iter().map(Into::into).collect())
}

#[derive(Debug, sqlx::FromRow)]
struct BillingPortalSubscriptionProviderRecord {
    provider: String,
    status: String,
    primary: bool,
    fallback_eligible: bool,
    activated_at: Option<DateTime<Utc>>,
    deactivated_at: Option<DateTime<Utc>>,
}

impl From<BillingPortalSubscriptionProviderRecord> for BillingPortalSubscriptionProviderView {
    fn from(record: BillingPortalSubscriptionProviderRecord) -> Self {
        Self {
            provider: record.provider,
            status: record.status,
            primary: record.primary,
            fallback_eligible: record.fallback_eligible,
            activated_at: record.activated_at,
            deactivated_at: record.deactivated_at,
        }
    }
}
