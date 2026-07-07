use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingEntitlementChangedEvent {
    pub workspace_id: Uuid,
    pub snapshot_id: Uuid,
    pub subscription_status: String,
    pub effective_at: DateTime<Utc>,
    pub entitlements: BillingEntitlementPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingEntitlementPayload {
    pub can_upload: bool,
    pub can_create_share_links: bool,
    pub included_storage_bytes: i64,
    pub included_users: i64,
    pub max_share_links: i64,
    pub billing_locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveEntitlementProjection {
    pub workspace_id: Uuid,
    pub source_event_id: String,
    pub status: String,
    pub features: Value,
    pub quotas: Value,
    pub billing_locked: bool,
    pub effective_at: DateTime<Utc>,
}

pub fn drive_entitlement_projection_from_event(
    event: BillingEntitlementChangedEvent,
) -> DriveEntitlementProjection {
    DriveEntitlementProjection {
        workspace_id: event.workspace_id,
        source_event_id: event.snapshot_id.to_string(),
        status: event.subscription_status,
        features: json!({
            "upload": event.entitlements.can_upload,
            "share_links": event.entitlements.can_create_share_links,
        }),
        quotas: json!({
            "storage_bytes": event.entitlements.included_storage_bytes,
            "included_users": event.entitlements.included_users,
            "max_share_links": event.entitlements.max_share_links,
        }),
        billing_locked: event.entitlements.billing_locked,
        effective_at: event.effective_at,
    }
}

pub async fn persist_entitlement_projection(
    db: &PgPool,
    projection: DriveEntitlementProjection,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO billing_entitlement_snapshots (
          workspace_id,
          source_event_id,
          status,
          features,
          quotas,
          billing_locked,
          effective_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (workspace_id, source_event_id)
        WHERE source_event_id IS NOT NULL
        DO UPDATE SET
          status = EXCLUDED.status,
          features = EXCLUDED.features,
          quotas = EXCLUDED.quotas,
          billing_locked = EXCLUDED.billing_locked,
          effective_at = EXCLUDED.effective_at
        "#,
    )
    .bind(projection.workspace_id)
    .bind(projection.source_event_id)
    .bind(projection.status)
    .bind(sqlx::types::Json(projection.features))
    .bind(sqlx::types::Json(projection.quotas))
    .bind(projection.billing_locked)
    .bind(projection.effective_at)
    .execute(db)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entitlement_event_maps_to_drive_projection() {
        let workspace_id = Uuid::nil();
        let snapshot_id = Uuid::from_u128(1);
        let effective_at = DateTime::from_timestamp(1_782_000_000, 0).unwrap();

        let projection = drive_entitlement_projection_from_event(BillingEntitlementChangedEvent {
            workspace_id,
            snapshot_id,
            subscription_status: "active".to_string(),
            effective_at,
            entitlements: BillingEntitlementPayload {
                can_upload: false,
                can_create_share_links: true,
                included_storage_bytes: 1_073_741_824,
                included_users: 3,
                max_share_links: 10,
                billing_locked: false,
            },
        });

        assert_eq!(projection.workspace_id, workspace_id);
        assert_eq!(projection.source_event_id, snapshot_id.to_string());
        assert_eq!(projection.status, "active");
        assert_eq!(projection.features["upload"], false);
        assert_eq!(projection.features["share_links"], true);
        assert_eq!(projection.quotas["storage_bytes"], 1_073_741_824);
    }

    #[test]
    fn suspended_entitlement_projection_preserves_billing_lock() {
        let projection = drive_entitlement_projection_from_event(BillingEntitlementChangedEvent {
            workspace_id: Uuid::nil(),
            snapshot_id: Uuid::from_u128(2),
            subscription_status: "suspended".to_string(),
            effective_at: DateTime::from_timestamp(1_782_000_000, 0).unwrap(),
            entitlements: BillingEntitlementPayload {
                can_upload: false,
                can_create_share_links: false,
                included_storage_bytes: 0,
                included_users: 0,
                max_share_links: 0,
                billing_locked: true,
            },
        });

        assert!(projection.billing_locked);
        assert_eq!(projection.features["upload"], false);
        assert_eq!(projection.quotas["storage_bytes"], 0);
    }
}
