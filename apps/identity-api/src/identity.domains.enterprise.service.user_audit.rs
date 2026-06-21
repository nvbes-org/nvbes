use sqlx::{Postgres, Transaction};
use std::collections::BTreeSet;
use uuid::Uuid;

use crate::domains::authz::AdminScope;
use crate::domains::enterprise::db;
use crate::http::error::AppError;

pub(super) async fn insert_member_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    scope: AdminScope,
    workspace_ids: &[Uuid],
    actor_id: Uuid,
    action: &'static str,
    target_user_id: Uuid,
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    match scope {
        AdminScope::Tenant => {
            db::insert_audit(
                tx,
                tenant_id,
                actor_id,
                action,
                "principal",
                Some(target_user_id),
                metadata,
            )
            .await
        }
        AdminScope::Organization(_) => {
            for workspace_id in workspace_ids {
                db::insert_workspace_audit(
                    tx,
                    tenant_id,
                    Some(*workspace_id),
                    actor_id,
                    action,
                    "principal",
                    Some(target_user_id),
                    metadata.clone(),
                )
                .await?;
            }
            Ok(())
        }
    }
}

pub(super) fn merge_workspace_ids(left: &[Uuid], right: &[Uuid]) -> Vec<Uuid> {
    left.iter()
        .chain(right.iter())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
