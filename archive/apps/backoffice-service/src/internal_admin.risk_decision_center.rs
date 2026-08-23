use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1 as billing_pb;

#[derive(Debug, Serialize)]
struct RiskDecisionSnapshot {
    identity_risk_event_count_24h: i64,
    high_identity_risk_event_count_24h: i64,
    billing_risk_signal_count_24h: i64,
    high_billing_risk_score_count: i64,
    active_access_policy_count: i64,
    access_policy_count_24h: i64,
    recent_identity_risks: Vec<IdentityRiskDecision>,
    billing_risk_scores: Vec<BillingRiskScore>,
    billing_risk_signals: Vec<BillingRiskSignal>,
    active_access_policies: Vec<AccessPolicySnapshot>,
}

#[derive(Debug, Serialize)]
struct IdentityRiskDecision {
    id: Uuid,
    principal_id: Uuid,
    email: Option<String>,
    tenant_id: Uuid,
    tenant_name: String,
    event_type: String,
    risk_score: f64,
    decision: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct BillingRiskScore {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    score: f64,
    decision: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct BillingRiskSignal {
    id: Uuid,
    tenant_id: Option<Uuid>,
    tenant_name: Option<String>,
    signal_type: String,
    occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AccessPolicySnapshot {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    workspace_id: Option<Uuid>,
    workspace_name: Option<String>,
    policy_state: String,
    reason: String,
    effective_from: DateTime<Utc>,
    effective_to: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/risk-decision-center",
        get(risk_decision_center_route),
    )
}

async fn risk_decision_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RiskDecisionSnapshot>, AppError> {
    let actor_principal_id = actor_principal_id(&headers)?;
    let identity = load_identity_risk_decision(&state.db).await?;
    let billing = crate::billing_grpc::get_admin_risk_decision_center(
        &state.billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id: Uuid::nil(),
            actor_principal_id,
        },
    )
    .await?;
    Ok(Json(risk_decision_from_parts(identity, billing)?))
}

struct IdentityRiskSnapshot {
    identity_risk_event_count_24h: i64,
    high_identity_risk_event_count_24h: i64,
    recent_identity_risks: Vec<IdentityRiskDecision>,
}

async fn load_identity_risk_decision(db: &PgPool) -> Result<IdentityRiskSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM risk_events WHERE created_at >= NOW() - INTERVAL '24 hours')
            AS identity_risk_event_count_24h,
          (
            SELECT COUNT(*) FROM risk_events
            WHERE created_at >= NOW() - INTERVAL '24 hours' AND risk_score >= 0.7
          ) AS high_identity_risk_event_count_24h
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(IdentityRiskSnapshot {
        identity_risk_event_count_24h: metrics.get("identity_risk_event_count_24h"),
        high_identity_risk_event_count_24h: metrics.get("high_identity_risk_event_count_24h"),
        recent_identity_risks: load_identity_risks(db).await?,
    })
}

async fn load_identity_risks(db: &PgPool) -> Result<Vec<IdentityRiskDecision>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT re.id, re.principal_id, u.email, p.tenant_id, t.name AS tenant_name,
          re.event_type, re.risk_score, re.decision, re.created_at
        FROM risk_events re
        JOIN principals p ON p.id = re.principal_id
        JOIN tenants t ON t.id = p.tenant_id
        LEFT JOIN users u ON u.principal_id = re.principal_id
        ORDER BY re.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| IdentityRiskDecision {
            id: row.get("id"),
            principal_id: row.get("principal_id"),
            email: row.get("email"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            event_type: row.get("event_type"),
            risk_score: row.get("risk_score"),
            decision: row.get("decision"),
            created_at: row.get("created_at"),
        })
        .collect())
}

fn risk_decision_from_parts(
    identity: IdentityRiskSnapshot,
    billing: billing_pb::AdminRiskDecisionCenterSnapshot,
) -> Result<RiskDecisionSnapshot, AppError> {
    Ok(RiskDecisionSnapshot {
        identity_risk_event_count_24h: identity.identity_risk_event_count_24h,
        high_identity_risk_event_count_24h: identity.high_identity_risk_event_count_24h,
        billing_risk_signal_count_24h: billing.billing_risk_signal_count_24h,
        high_billing_risk_score_count: billing.high_billing_risk_score_count,
        active_access_policy_count: billing.active_access_policy_count,
        access_policy_count_24h: billing.access_policy_count_24h,
        recent_identity_risks: identity.recent_identity_risks,
        billing_risk_scores: billing
            .billing_risk_scores
            .into_iter()
            .map(billing_risk_score_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        billing_risk_signals: billing
            .billing_risk_signals
            .into_iter()
            .map(billing_risk_signal_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        active_access_policies: billing
            .active_access_policies
            .into_iter()
            .map(access_policy_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn billing_risk_score_from_grpc(
    value: billing_pb::AdminBillingRiskScore,
) -> Result<BillingRiskScore, AppError> {
    Ok(BillingRiskScore {
        id: parse_uuid(&value.id, "billing risk score id")?,
        tenant_id: parse_uuid(&value.tenant_id, "billing risk score tenant_id")?,
        tenant_name: value.tenant_name,
        score: value.score,
        decision: value.decision,
        created_at: parse_datetime(&value.created_at, "billing risk score created_at")?,
    })
}

fn billing_risk_signal_from_grpc(
    value: billing_pb::AdminBillingRiskSignal,
) -> Result<BillingRiskSignal, AppError> {
    Ok(BillingRiskSignal {
        id: parse_uuid(&value.id, "billing risk signal id")?,
        tenant_id: parse_optional_uuid(&value.tenant_id, "billing risk signal tenant_id")?,
        tenant_name: empty_to_none(value.tenant_name),
        signal_type: value.signal_type,
        occurred_at: parse_datetime(&value.occurred_at, "billing risk signal occurred_at")?,
    })
}

fn access_policy_from_grpc(
    value: billing_pb::AdminAccessPolicySnapshot,
) -> Result<AccessPolicySnapshot, AppError> {
    Ok(AccessPolicySnapshot {
        id: parse_uuid(&value.id, "access policy id")?,
        tenant_id: parse_uuid(&value.tenant_id, "access policy tenant_id")?,
        tenant_name: value.tenant_name,
        workspace_id: parse_optional_uuid(&value.workspace_id, "access policy workspace_id")?,
        workspace_name: empty_to_none(value.workspace_name),
        policy_state: value.policy_state,
        reason: value.reason,
        effective_from: parse_datetime(&value.effective_from, "access policy effective_from")?,
        effective_to: parse_optional_datetime(&value.effective_to, "access policy effective_to")?,
    })
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_optional_datetime(
    value: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_datetime(value, field).map(Some)
    }
}

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
