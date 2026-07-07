use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub(crate) struct AuditQuery {
    #[serde(default = "default_limit")]
    pub(crate) limit: i64,
    pub(crate) action: Option<String>,
    pub(crate) target_type: Option<String>,
    pub(crate) q: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuditEvent {
    pub(crate) id: Uuid,
    pub(crate) tenant_id: Uuid,
    pub(crate) workspace_id: Option<Uuid>,
    pub(crate) action: String,
    pub(crate) actor_principal_id: Option<Uuid>,
    pub(crate) actor_email: Option<String>,
    pub(crate) target_type: String,
    pub(crate) target_id: Option<Uuid>,
    pub(crate) target_link: Option<AuditTargetLink>,
    pub(crate) changes: Vec<AuditChange>,
    pub(crate) metadata: Value,
    pub(crate) event_hash: String,
    pub(crate) previous_event_hash: Option<String>,
    pub(crate) hash_chain_status: &'static str,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct AuditTargetLink {
    pub(crate) kind: &'static str,
    pub(crate) id: Uuid,
    pub(crate) href: &'static str,
    pub(crate) label: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct AuditChange {
    pub(crate) field: String,
    pub(crate) before: Value,
    pub(crate) after: Value,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/workspaces/{workspaceId}/admin/audit-events",
        get(list_audit_events_route),
    )
}

async fn list_audit_events_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEvent>>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    let limit = query.limit.clamp(1, 100);
    Ok(Json(
        list_audit_events(&state.db, access.tenant_id, query, limit).await?,
    ))
}

pub(crate) async fn list_audit_events(
    db: &PgPool,
    tenant_id: Uuid,
    query: AuditQuery,
    limit: i64,
) -> Result<Vec<AuditEvent>, AppError> {
    let search_pattern = query.q.as_ref().map(|value| format!("%{}%", value.trim()));
    let rows = sqlx::query(
        r#"
        SELECT ae.id, ae.tenant_id, ae.workspace_id, ae.action, ae.actor_principal_id,
          u.email AS actor_email, ae.target_type, ae.target_id, ae.metadata, ae.event_hash,
          ae.previous_event_hash, ae.created_at
        FROM audit_events ae
        LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
        WHERE ae.tenant_id = $1
          AND ($2::text IS NULL OR ae.action = $2)
          AND ($3::text IS NULL OR ae.target_type = $3)
          AND (
            $4::text IS NULL
            OR ae.action ILIKE $4
            OR ae.target_type ILIKE $4
            OR ae.target_id::text ILIKE $4
            OR ae.actor_principal_id::text ILIKE $4
            OR u.email ILIKE $4
          )
        ORDER BY ae.created_at DESC, ae.id DESC
        LIMIT $5
        "#,
    )
    .bind(tenant_id)
    .bind(trimmed_filter(query.action))
    .bind(trimmed_filter(query.target_type))
    .bind(search_pattern)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let tenant_id = row.get("tenant_id");
            let workspace_id = row.get("workspace_id");
            let target_type = row.get::<String, _>("target_type");
            let target_id = row.get("target_id");
            let metadata = row.get::<Value, _>("metadata");
            let event_hash = row.get::<String, _>("event_hash");
            let previous_event_hash: Option<String> = row.get("previous_event_hash");
            AuditEvent {
                id: row.get("id"),
                tenant_id,
                workspace_id,
                action: row.get("action"),
                actor_principal_id: row.get("actor_principal_id"),
                actor_email: row.get("actor_email"),
                target_link: target_link(&target_type, target_id, tenant_id, workspace_id),
                changes: audit_changes(&metadata),
                target_type,
                target_id,
                metadata,
                hash_chain_status: hash_chain_status(&event_hash, previous_event_hash.as_deref()),
                event_hash,
                previous_event_hash,
                created_at: row.get("created_at"),
            }
        })
        .collect())
}

fn default_limit() -> i64 {
    25
}

fn trimmed_filter(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn audit_changes(metadata: &Value) -> Vec<AuditChange> {
    metadata
        .get("changes")
        .and_then(Value::as_array)
        .map(|changes| {
            changes
                .iter()
                .filter_map(|change| {
                    let field = change.get("field")?.as_str()?.trim();
                    if field.is_empty() {
                        return None;
                    }
                    Some(AuditChange {
                        field: field.to_string(),
                        before: change.get("before").cloned().unwrap_or(Value::Null),
                        after: change.get("after").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn hash_chain_status(event_hash: &str, previous_event_hash: Option<&str>) -> &'static str {
    if event_hash.trim().is_empty() || event_hash == "backfill" {
        return "hash_anomaly";
    }
    if previous_event_hash
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        return "linked";
    }
    "chain_head"
}

fn target_link(
    target_type: &str,
    target_id: Option<Uuid>,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
) -> Option<AuditTargetLink> {
    if target_type == "tenant" {
        return Some(link(
            "tenant",
            target_id.unwrap_or(tenant_id),
            "#tenant-detail",
        ));
    }
    if target_type == "workspace" {
        return target_id
            .or(workspace_id)
            .map(|id| link("workspace", id, "#workspace-detail"));
    }
    if matches!(
        target_type,
        "principal" | "user" | "mfa_factor" | "oauth_consent"
    ) {
        return target_id.map(|id| link("user", id, "#user-detail"));
    }
    workspace_id.map(|id| link("workspace", id, "#workspace-detail"))
}

fn link(kind: &'static str, id: Uuid, href: &'static str) -> AuditTargetLink {
    AuditTargetLink {
        kind,
        id,
        href,
        label: format!("{kind}:{id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimmed_filter_discards_blank_values() {
        assert_eq!(
            trimmed_filter(Some("  action  ".to_string())),
            Some("action".to_string())
        );
        assert_eq!(trimmed_filter(Some("   ".to_string())), None);
        assert_eq!(trimmed_filter(None), None);
    }

    #[test]
    fn audit_changes_extracts_normalized_before_after_diff() {
        let metadata = serde_json::json!({
            "changes": [
                {"field": "status", "before": "active", "after": "suspended"},
                {"field": "", "before": 1, "after": 2},
                {"before": "ignored", "after": "ignored"}
            ]
        });

        assert_eq!(
            audit_changes(&metadata),
            vec![AuditChange {
                field: "status".to_string(),
                before: serde_json::json!("active"),
                after: serde_json::json!("suspended"),
            }]
        );
    }

    #[test]
    fn target_link_prefers_object_detail_when_supported() {
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        assert_eq!(
            target_link("tenant", None, tenant_id, Some(workspace_id))
                .map(|link| (link.kind, link.id, link.href)),
            Some(("tenant", tenant_id, "#tenant-detail"))
        );
        assert_eq!(
            target_link("principal", Some(user_id), tenant_id, Some(workspace_id))
                .map(|link| (link.kind, link.id, link.href)),
            Some(("user", user_id, "#user-detail"))
        );
        assert_eq!(
            target_link("billing_invoice", None, tenant_id, Some(workspace_id))
                .map(|link| (link.kind, link.id, link.href)),
            Some(("workspace", workspace_id, "#workspace-detail"))
        );
    }

    #[test]
    fn hash_chain_status_distinguishes_linked_head_and_anomaly() {
        assert_eq!(hash_chain_status("hash", Some("previous")), "linked");
        assert_eq!(hash_chain_status("hash", None), "chain_head");
        assert_eq!(hash_chain_status("backfill", None), "hash_anomaly");
        assert_eq!(hash_chain_status("", None), "hash_anomaly");
    }
}
