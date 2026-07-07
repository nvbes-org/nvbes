use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, header},
    response::IntoResponse,
    routing::{get, post},
};
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_idempotency_key, require_operator_role_grant, require_permission,
    require_strong_confirmation, require_strong_confirmation_for_value,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::authorize_backoffice;
use crate::billing_admin_exports::{build_finance_export, parse_finance_export_type};
use crate::billing_admin_mutations::{
    create_credit_note, create_manual_compensation, create_provider_migration,
    create_refund_intent, create_write_off, override_grace_period, replay_provider_event,
};
use crate::billing_admin_search::search_billing_admin;
use crate::billing_admin_types::{
    CreditNoteRequest, GraceOverrideRequest, ManualCompRequest, MutationResult,
    ProviderMigrationRequest, ProviderReplayRequest, ProviderReplayResult, RefundIntentRequest,
    SearchQuery, SearchResult,
};
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{workspaceId}/billing/admin/search",
            get(search_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/credit-notes",
            post(credit_note_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/write-offs",
            post(write_off_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/refund-intents",
            post(refund_intent_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/provider-events/replay",
            post(replay_provider_event_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/provider-migrations",
            post(provider_migration_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/grace-overrides",
            post(grace_override_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/manual-compensations",
            post(manual_comp_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/exports/{exportType}",
            post(finance_export_route),
        )
}

async fn search_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        search_billing_admin(&state.db, access.tenant_id, &query.q).await?,
    ))
}

async fn credit_note_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreditNoteRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_billing_mutation(
        &headers,
        &request.confirm_code,
        "CREATE CREDIT NOTE",
        request.invoice_id,
    )?;
    require_operator_role_grant(&state.db, &headers).await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        create_credit_note(
            &state.db,
            &state.billing_grpc_endpoint,
            access,
            workspace_id,
            request,
        )
        .await?,
    ))
}

async fn write_off_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreditNoteRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_billing_mutation(
        &headers,
        &request.confirm_code,
        "WRITE OFF",
        request.invoice_id,
    )?;
    require_operator_role_grant(&state.db, &headers).await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        create_write_off(
            &state.db,
            &state.billing_grpc_endpoint,
            access,
            workspace_id,
            request,
        )
        .await?,
    ))
}

async fn refund_intent_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<RefundIntentRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_billing_mutation(
        &headers,
        &request.confirm_code,
        "CREATE REFUND",
        request.payment_id,
    )?;
    require_operator_role_grant(&state.db, &headers).await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        create_refund_intent(
            &state.db,
            &state.billing_grpc_endpoint,
            access,
            workspace_id,
            request,
        )
        .await?,
    ))
}

async fn replay_provider_event_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ProviderReplayRequest>,
) -> Result<Json<ProviderReplayResult>, AppError> {
    require_billing_mutation_for_value(
        &headers,
        &request.confirm_code,
        "REPLAY EVENT",
        &request.provider_event_id,
    )?;
    require_operator_role_grant(&state.db, &headers).await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        replay_provider_event(
            &state.db,
            &state.billing_grpc_endpoint,
            access,
            workspace_id,
            request,
        )
        .await?,
    ))
}

async fn provider_migration_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ProviderMigrationRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_billing_mutation_for_value(
        &headers,
        &request.confirm_code,
        "PLAN MIGRATION",
        &provider_migration_target(&request),
    )?;
    require_operator_role_grant(&state.db, &headers).await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        create_provider_migration(
            &state.db,
            &state.billing_grpc_endpoint,
            access,
            workspace_id,
            request,
        )
        .await?,
    ))
}

async fn grace_override_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<GraceOverrideRequest>,
) -> Result<Json<MutationResult>, AppError> {
    match request.subscription_id {
        Some(subscription_id) => require_billing_mutation(
            &headers,
            &request.confirm_code,
            "OVERRIDE GRACE",
            subscription_id,
        )?,
        None => require_billing_mutation(
            &headers,
            &request.confirm_code,
            "OVERRIDE GRACE",
            workspace_id,
        )?,
    }
    require_operator_role_grant(&state.db, &headers).await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        override_grace_period(
            &state.db,
            &state.billing_grpc_endpoint,
            access,
            workspace_id,
            request,
        )
        .await?,
    ))
}

async fn manual_comp_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ManualCompRequest>,
) -> Result<Json<MutationResult>, AppError> {
    require_billing_mutation(
        &headers,
        &request.confirm_code,
        "CREATE COMPENSATION",
        workspace_id,
    )?;
    require_operator_role_grant(&state.db, &headers).await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        create_manual_compensation(
            &state.db,
            &state.billing_grpc_endpoint,
            access,
            workspace_id,
            request,
        )
        .await?,
    ))
}

async fn finance_export_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, export_type)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, AppError> {
    require_idempotency_key(&headers)?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    let export_type = parse_finance_export_type(&export_type)?;
    let export = build_finance_export(&state.db, access.tenant_id, export_type).await?;
    Ok((
        [
            (header::CONTENT_TYPE, export.content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", export.filename),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
            (
                HeaderName::from_static("x-nvbes-billing-export-run-id"),
                export.export_run_id.to_string(),
            ),
            (
                HeaderName::from_static("x-nvbes-billing-export-row-count"),
                export.row_count.to_string(),
            ),
        ],
        export.body,
    ))
}

fn require_billing_mutation(
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: Uuid,
) -> Result<(), AppError> {
    require_billing_authorization(headers)?;
    require_strong_confirmation(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)
}

fn require_billing_mutation_for_value(
    headers: &HeaderMap,
    confirm_code: &str,
    expected_code: &str,
    target_id: &str,
) -> Result<(), AppError> {
    require_billing_authorization(headers)?;
    require_strong_confirmation_for_value(confirm_code, expected_code, target_id)?;
    require_dual_control(headers)
}

fn require_billing_authorization(headers: &HeaderMap) -> Result<(), AppError> {
    require_idempotency_key(headers)?;
    require_permission(headers, BackofficePermission::BillingMutate)
}

fn provider_migration_target(request: &ProviderMigrationRequest) -> String {
    format!(
        "{}->{}",
        request.from_provider.trim(),
        request.to_provider.trim()
    )
}
