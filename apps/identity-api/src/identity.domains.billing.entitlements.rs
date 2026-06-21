use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use nvbes_billing::types::ProductEntitlementsView;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::db;
use super::service::get_billing;

pub async fn get_entitlements_for_workspace(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<nvbes_billing::types::ProductEntitlementsView, AppError> {
    let billing_response = get_billing(db, workspace_id).await?;
    Ok(billing_response.entitlements)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingEntitlementChangedEvent {
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub snapshot_id: Uuid,
    pub plan_code: String,
    pub subscription_status: String,
    pub effective_at: DateTime<Utc>,
    pub entitlements: ProductEntitlementsView,
    pub reason: Option<String>,
    pub metadata: Value,
}

pub struct EntitlementSnapshotInput {
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub source_subscription_id: Option<Uuid>,
    pub plan_code: String,
    pub subscription_status: String,
    pub entitlements: ProductEntitlementsView,
    pub reason: Option<String>,
    pub effective_at: DateTime<Utc>,
    pub metadata: Value,
}

pub struct PersistedEntitlementChange {
    pub snapshot_id: Uuid,
    pub event_id: String,
    pub event: BillingEntitlementChangedEvent,
}

pub async fn persist_entitlement_snapshot_change(
    tx: &mut Transaction<'_, Postgres>,
    input: EntitlementSnapshotInput,
) -> Result<PersistedEntitlementChange, AppError> {
    let features = entitlement_features_json(&input.entitlements);
    let quotas = entitlement_quotas_json(&input.entitlements);
    let snapshot_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO billing_entitlement_snapshots (
          tenant_id,
          workspace_id,
          source_subscription_id,
          status,
          features,
          quotas,
          effective_from
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.workspace_id)
    .bind(input.source_subscription_id)
    .bind(&input.subscription_status)
    .bind(sqlx::types::Json(&features))
    .bind(sqlx::types::Json(&quotas))
    .bind(input.effective_at)
    .fetch_one(&mut **tx)
    .await?;

    let event = build_entitlement_changed_event(&input, snapshot_id);
    let event_id = entitlement_change_event_id(input.tenant_id, input.workspace_id, snapshot_id);
    sqlx::query(
        r#"
        INSERT INTO billing_entitlement_changes (
          tenant_id,
          snapshot_id,
          event_id,
          diff,
          published_at
        )
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(input.tenant_id)
    .bind(snapshot_id)
    .bind(&event_id)
    .bind(sqlx::types::Json(json!({
        "reason": input.reason,
        "metadata": input.metadata,
        "features": features,
        "quotas": quotas,
    })))
    .execute(&mut **tx)
    .await?;

    Ok(PersistedEntitlementChange {
        snapshot_id,
        event_id,
        event,
    })
}

pub async fn persist_current_workspace_entitlements_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    source_subscription_id: Option<Uuid>,
    reason: &str,
    metadata: Value,
) -> Result<PersistedEntitlementChange, AppError> {
    let tenant_id = db::tenant_id_for_workspace(tx, workspace_id).await?;
    let record = db::fetch_billing_state_tx(tx, workspace_id).await?;
    let entitlements = nvbes_billing::entitlements_view(&record);
    persist_entitlement_snapshot_change(
        tx,
        EntitlementSnapshotInput {
            tenant_id,
            workspace_id,
            source_subscription_id,
            plan_code: record.plan_code,
            subscription_status: record.subscription_status,
            entitlements,
            reason: Some(reason.to_string()),
            effective_at: Utc::now(),
            metadata,
        },
    )
    .await
}

pub fn build_entitlement_changed_event(
    input: &EntitlementSnapshotInput,
    snapshot_id: Uuid,
) -> BillingEntitlementChangedEvent {
    BillingEntitlementChangedEvent {
        tenant_id: input.tenant_id,
        workspace_id: input.workspace_id,
        snapshot_id,
        plan_code: input.plan_code.clone(),
        subscription_status: input.subscription_status.clone(),
        effective_at: input.effective_at,
        entitlements: input.entitlements.clone(),
        reason: input.reason.clone(),
        metadata: input.metadata.clone(),
    }
}

pub fn entitlement_change_event_id(
    tenant_id: Uuid,
    workspace_id: Uuid,
    snapshot_id: Uuid,
) -> String {
    format!("billing.entitlement.changed:{tenant_id}:{workspace_id}:{snapshot_id}")
}

fn entitlement_features_json(entitlements: &ProductEntitlementsView) -> Value {
    json!({
        "upload": entitlements.can_upload,
        "share_links": entitlements.can_create_share_links,
        "billing_locked": entitlements.billing_locked,
        "audit_level": entitlements.audit_level,
    })
}

fn entitlement_quotas_json(entitlements: &ProductEntitlementsView) -> Value {
    json!({
        "storage_bytes": entitlements.included_storage_bytes,
        "included_users": entitlements.included_users,
        "max_share_links": entitlements.max_share_links,
        "max_share_link_ttl_days": entitlements.max_share_link_ttl_days,
        "api_key_limit": entitlements.api_key_limit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entitlements() -> ProductEntitlementsView {
        ProductEntitlementsView {
            can_upload: true,
            can_create_share_links: false,
            included_storage_bytes: 1_073_741_824,
            included_users: 3,
            max_share_links: 0,
            max_share_link_ttl_days: 0,
            audit_level: "standard".to_string(),
            api_key_limit: 5,
            billing_locked: false,
        }
    }

    #[test]
    fn entitlement_changed_event_matches_contract_payload_shape() {
        let tenant_id = Uuid::from_u128(1);
        let workspace_id = Uuid::from_u128(2);
        let snapshot_id = Uuid::from_u128(3);
        let effective_at = DateTime::from_timestamp(1_782_000_000, 0).unwrap();
        let input = EntitlementSnapshotInput {
            tenant_id,
            workspace_id,
            source_subscription_id: None,
            plan_code: "team".to_string(),
            subscription_status: "active".to_string(),
            entitlements: entitlements(),
            reason: Some("subscription_changed".to_string()),
            effective_at,
            metadata: json!({ "source": "unit-test" }),
        };

        let event = build_entitlement_changed_event(&input, snapshot_id);

        assert_eq!(event.tenant_id, tenant_id);
        assert_eq!(event.workspace_id, workspace_id);
        assert_eq!(event.snapshot_id, snapshot_id);
        assert_eq!(event.plan_code, "team");
        assert_eq!(event.subscription_status, "active");
        assert_eq!(event.entitlements.included_storage_bytes, 1_073_741_824);
        assert_eq!(event.reason.as_deref(), Some("subscription_changed"));
    }

    #[test]
    fn entitlement_change_event_id_is_deterministic() {
        let event_id =
            entitlement_change_event_id(Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));

        assert_eq!(
            event_id,
            "billing.entitlement.changed:00000000-0000-0000-0000-000000000001:00000000-0000-0000-0000-000000000002:00000000-0000-0000-0000-000000000003"
        );
    }
}
