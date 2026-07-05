use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::types::ProductEntitlementsView;

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

pub async fn persist_current_workspace_entitlements_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    source_subscription_id: Option<Uuid>,
    reason: &str,
    metadata: Value,
) -> Result<PersistedEntitlementChange, sqlx::Error> {
    let tenant_id = crate::db::tenant_id_for_workspace_tx(tx, workspace_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    let record = crate::db::fetch_billing_state_tx(tx, workspace_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    let entitlements = crate::entitlements_view(&record);

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

pub async fn persist_entitlement_snapshot_change(
    tx: &mut Transaction<'_, Postgres>,
    input: EntitlementSnapshotInput,
) -> Result<PersistedEntitlementChange, sqlx::Error> {
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
    .fetch_one(tx.as_mut())
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
    .execute(tx.as_mut())
    .await?;

    Ok(PersistedEntitlementChange {
        snapshot_id,
        event_id,
        event,
    })
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
