use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;
use crate::audit::{AuditEvent, AuditQuery, list_audit_events};
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;

const EXPORT_LIMIT: i64 = 500;

#[derive(Debug, Deserialize)]
struct EvidenceExportQuery {
    action: Option<String>,
    target_type: Option<String>,
    q: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuditEvidenceExport {
    export_id: Uuid,
    generated_at: DateTime<Utc>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    event_count: usize,
    filters: AuditEvidenceExportFilters,
    hash_chain: AuditEvidenceHashChain,
    events: Vec<AuditEvent>,
}

#[derive(Debug, Serialize)]
struct AuditEvidenceExportFilters {
    action: Option<String>,
    target_type: Option<String>,
    q: Option<String>,
    limit: i64,
}

#[derive(Debug, Serialize, PartialEq)]
struct AuditEvidenceHashChain {
    head_event_hash: Option<String>,
    tail_event_hash: Option<String>,
    anomaly_count: usize,
    linked_count: usize,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/workspaces/{workspaceId}/admin/audit-evidence/export",
        get(export_audit_evidence_route),
    )
}

async fn export_audit_evidence_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<EvidenceExportQuery>,
) -> Result<Json<AuditEvidenceExport>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    let filters = AuditEvidenceExportFilters {
        action: trim_filter(query.action),
        target_type: trim_filter(query.target_type),
        q: trim_filter(query.q),
        limit: EXPORT_LIMIT,
    };
    let events = list_audit_events(
        &state.db,
        access.tenant_id,
        AuditQuery {
            limit: EXPORT_LIMIT,
            action: filters.action.clone(),
            target_type: filters.target_type.clone(),
            q: filters.q.clone(),
        },
        EXPORT_LIMIT,
    )
    .await?;

    Ok(Json(AuditEvidenceExport {
        export_id: Uuid::new_v4(),
        generated_at: Utc::now(),
        tenant_id: access.tenant_id,
        workspace_id,
        event_count: events.len(),
        filters,
        hash_chain: evidence_hash_chain(&events),
        events,
    }))
}

fn evidence_hash_chain(events: &[AuditEvent]) -> AuditEvidenceHashChain {
    AuditEvidenceHashChain {
        head_event_hash: events.first().map(|event| event.event_hash.clone()),
        tail_event_hash: events.last().map(|event| event.event_hash.clone()),
        anomaly_count: events
            .iter()
            .filter(|event| event.hash_chain_status == "hash_anomaly")
            .count(),
        linked_count: events
            .iter()
            .filter(|event| event.hash_chain_status == "linked")
            .count(),
    }
}

fn trim_filter(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn evidence_hash_chain_summarizes_export_integrity() {
        let tenant_id = Uuid::new_v4();
        let events = vec![
            audit_event(tenant_id, "hash-2", Some("hash-1"), "linked"),
            audit_event(tenant_id, "backfill", None, "hash_anomaly"),
        ];

        assert_eq!(
            evidence_hash_chain(&events),
            AuditEvidenceHashChain {
                head_event_hash: Some("hash-2".to_string()),
                tail_event_hash: Some("backfill".to_string()),
                anomaly_count: 1,
                linked_count: 1,
            }
        );
    }

    #[test]
    fn trim_filter_discards_blank_export_filters() {
        assert_eq!(
            trim_filter(Some(" status ".to_string())),
            Some("status".to_string())
        );
        assert_eq!(trim_filter(Some(" ".to_string())), None);
    }

    fn audit_event(
        tenant_id: Uuid,
        event_hash: &str,
        previous_event_hash: Option<&str>,
        hash_chain_status: &'static str,
    ) -> AuditEvent {
        AuditEvent {
            id: Uuid::new_v4(),
            tenant_id,
            workspace_id: None,
            action: "internal_admin.test".to_string(),
            actor_principal_id: None,
            actor_email: None,
            target_type: "tenant".to_string(),
            target_id: Some(tenant_id),
            target_link: None,
            changes: Vec::new(),
            metadata: json!({}),
            event_hash: event_hash.to_string(),
            previous_event_hash: previous_event_hash.map(str::to_string),
            hash_chain_status,
            created_at: Utc::now(),
        }
    }
}
