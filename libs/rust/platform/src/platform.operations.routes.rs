use crate::{
    cockpit_auth::Permission,
    cockpit_server::PlatformCockpitState,
    operations_db::load_case,
    operations_error::OperationsError,
    operations_model::{Command, Receipt},
    operations_service::execute,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

fn authorize(
    state: &PlatformCockpitState,
    headers: &HeaderMap,
    permission: Permission,
) -> Result<crate::cockpit_auth::OperatorSession, OperationsError> {
    let actor = state
        .auth_policy
        .authenticate_headers(headers)
        .map_err(|_| OperationsError::Forbidden)?;
    if !actor.permits(permission) {
        return Err(OperationsError::Forbidden);
    }
    Ok(actor)
}

#[derive(Default, Deserialize)]
pub struct Page {
    #[serde(default)]
    after: i64,
    case_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct CasePage {
    after: Option<Uuid>,
}

pub async fn command(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
    Json(command): Json<Command>,
) -> Result<Json<Receipt>, OperationsError> {
    let actor = authorize(&state, &headers, Permission::WriteCases)?;
    Ok(Json(execute(&state.db, &actor, command).await?))
}

pub async fn cases(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
    Query(page): Query<CasePage>,
) -> Result<Json<Value>, OperationsError> {
    authorize(&state, &headers, Permission::ReadCases)?;
    let rows: Vec<(Uuid, Value)> = sqlx::query_as("SELECT id,document FROM operations_cases WHERE ($1::uuid IS NULL OR id>$1) ORDER BY id LIMIT 100")
        .bind(page.after).fetch_all(&state.db).await?;
    Ok(Json(
        json!({"next_after": rows.last().map(|r| r.0), "items": rows.into_iter().map(|r| r.1).collect::<Vec<_>>()}),
    ))
}

pub async fn case_context(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, OperationsError> {
    authorize(&state, &headers, Permission::ReadCases)?;
    let case = load_case(&state.db, id).await?;
    let observations: Vec<Value> = sqlx::query_scalar("SELECT DISTINCT ON (request->'action'->>'service') jsonb_build_object('observation',request->'action','recorded_by',actor,'recorded_at',created_at) FROM operations_audit WHERE case_id=$1 AND request->'action'->>'type'='record_observation' ORDER BY request->'action'->>'service', sequence DESC")
        .bind(id).fetch_all(&state.db).await?;
    Ok(Json(
        json!({"case": case, "services": state.context.snapshot().await,
        "observations":observations,"observation_source":"Manually verified owner API evidence; timestamps must be checked before action",
        "audit_url": format!("/api/v1/audits?case_id={id}"), "domain_mutations": "unavailable"}),
    ))
}

pub async fn audits(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
    Query(page): Query<Page>,
) -> Result<Json<Value>, OperationsError> {
    authorize(&state, &headers, Permission::ReadAudit)?;
    let rows: Vec<(i64, Value, Value)> = sqlx::query_as("SELECT sequence,request,receipt FROM operations_audit WHERE sequence>$1 AND ($2::uuid IS NULL OR case_id=$2) ORDER BY sequence LIMIT 100")
        .bind(page.after.max(0)).bind(page.case_id).fetch_all(&state.db).await?;
    Ok(Json(
        json!({"next_after": rows.last().map(|r| r.0), "items": rows.into_iter().map(|r| json!({"sequence":r.0,"command":r.1,"receipt":r.2})).collect::<Vec<_>>()}),
    ))
}

#[derive(Deserialize)]
pub struct CostQuery {
    month: NaiveDate,
}

pub async fn costs(
    State(state): State<PlatformCockpitState>,
    headers: HeaderMap,
    Query(query): Query<CostQuery>,
) -> Result<Json<Value>, OperationsError> {
    authorize(&state, &headers, Permission::ReadAudit)?;
    let rows: Vec<Value> = sqlx::query_scalar("SELECT c.document FROM operations_costs c WHERE month=$1 AND NOT EXISTS(SELECT 1 FROM operations_costs newer WHERE newer.document->>'replaces'=c.id::text) ORDER BY id LIMIT 1000")
        .bind(query.month).fetch_all(&state.db).await?;
    // Aggregate separately: never silently truncate totals when the detail page is full.
    let totals: (i64, i64, i64) = sqlx::query_as("SELECT coalesce(sum((c.document->>'actual_cents')::bigint),0)::bigint, coalesce(sum((c.document->>'forecast_cents')::bigint),0)::bigint, count(*) FROM operations_costs c WHERE month=$1 AND NOT EXISTS(SELECT 1 FROM operations_costs newer WHERE newer.document->>'replaces'=c.id::text)")
        .bind(query.month).fetch_one(&state.db).await?;
    Ok(Json(
        json!({"month":query.month,"currency":"EUR","tax_included":true,"items":rows,
        "actual_cents":totals.0,"forecast_cents":totals.1,"entries":totals.2,
        "target_cents":2000,"hard_limit_cents":3000,
        "review": if totals.2 == 0 { "unknown" } else if totals.1 >= 3000 { "stop" } else if totals.1 >= 2800 { "critical" } else if totals.1 >= 2500 { "economy" } else { "within_recorded_budget" },
        "coverage":"Operator must verify all providers; recorded totals are not proof of completeness"}),
    ))
}
