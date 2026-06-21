use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;
use crate::domains::auth::verification;
use crate::domains::authz::WorkspaceAccess;
use crate::domains::authz::{ResourceContext, WorkspaceAction, authorize_workspace_action};
use crate::domains::billing::service_admin::{
    BillingAdminMutationInput, BillingAdminMutationResult, BillingProviderEventReplayInput,
    BillingProviderEventReplayResult, BillingProviderMigrationInput, BillingRefundIntentInput,
    BillingRunbook, create_credit_note, create_provider_migration_run, create_refund_intent,
    create_write_off, replay_provider_event, runbook_slug,
};
use crate::http::error::AppError;
use nvbes_core::auth::Aal;

#[path = "identity.domains.billing.routes.admin.exports.rs"]
mod exports;
#[path = "identity.domains.billing.routes.admin.operations.rs"]
mod operations;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/billing/runbooks", get(list_billing_runbooks))
        .merge(exports::router())
        .merge(operations::router())
        .route(
            "/workspaces/{workspaceId}/billing/admin/credit-notes",
            post(create_credit_note_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/write-offs",
            post(create_write_off_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/refund-intents",
            post(create_refund_intent_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/provider-events/replay",
            post(replay_provider_event_route),
        )
        .route(
            "/workspaces/{workspaceId}/billing/admin/provider-migrations",
            post(create_provider_migration_route),
        )
}

#[derive(Debug, Serialize)]
pub struct BillingRunbookView {
    pub slug: &'static str,
    pub title: &'static str,
}

pub async fn list_billing_runbooks() -> Json<Vec<BillingRunbookView>> {
    Json(vec![
        BillingRunbookView {
            slug: runbook_slug(BillingRunbook::PspOutage),
            title: "PSP outage",
        },
        BillingRunbookView {
            slug: runbook_slug(BillingRunbook::WebhookLag),
            title: "Webhook lag",
        },
        BillingRunbookView {
            slug: runbook_slug(BillingRunbook::DuplicatePayment),
            title: "Duplicate payment",
        },
        BillingRunbookView {
            slug: runbook_slug(BillingRunbook::TaxConfigError),
            title: "Tax config error",
        },
        BillingRunbookView {
            slug: runbook_slug(BillingRunbook::LedgerImbalance),
            title: "Ledger imbalance",
        },
        BillingRunbookView {
            slug: runbook_slug(BillingRunbook::FailedExport),
            title: "Failed export",
        },
    ])
}

#[derive(Debug, Deserialize)]
pub struct CreditNoteRequest {
    pub invoice_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RefundIntentRequest {
    pub payment_id: Uuid,
    pub provider: String,
    pub amount_minor: i64,
    pub currency: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ProviderEventReplayRequest {
    pub provider: String,
    pub provider_event_id: String,
    pub reason: String,
}

#[derive(Deserialize)]
pub struct ProviderMigrationRequest {
    pub from_provider: String,
    pub to_provider: String,
    pub reason: String,
}

pub async fn create_credit_note_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreditNoteRequest>,
) -> Result<Json<BillingAdminMutationResult>, AppError> {
    let access = authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = require_tenant_id(access.tenant_id)?;

    let result = create_credit_note(
        &state.db,
        BillingAdminMutationInput {
            tenant_id,
            actor_principal_id: access.auth.user_id,
            invoice_id: request.invoice_id,
            amount_minor: request.amount_minor,
            currency: request.currency,
            reason: request.reason,
        },
    )
    .await?;

    Ok(Json(result))
}

pub async fn create_write_off_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreditNoteRequest>,
) -> Result<Json<BillingAdminMutationResult>, AppError> {
    let access = authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = require_tenant_id(access.tenant_id)?;

    let result = create_write_off(
        &state.db,
        BillingAdminMutationInput {
            tenant_id,
            actor_principal_id: access.auth.user_id,
            invoice_id: request.invoice_id,
            amount_minor: request.amount_minor,
            currency: request.currency,
            reason: request.reason,
        },
    )
    .await?;

    Ok(Json(result))
}

pub async fn create_refund_intent_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<RefundIntentRequest>,
) -> Result<Json<BillingAdminMutationResult>, AppError> {
    let access = authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = require_tenant_id(access.tenant_id)?;

    let result = create_refund_intent(
        &state.db,
        BillingRefundIntentInput {
            tenant_id,
            actor_principal_id: access.auth.user_id,
            payment_id: request.payment_id,
            provider: request.provider,
            amount_minor: request.amount_minor,
            currency: request.currency,
            reason: request.reason,
        },
    )
    .await?;

    Ok(Json(result))
}

pub async fn replay_provider_event_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ProviderEventReplayRequest>,
) -> Result<Json<BillingProviderEventReplayResult>, AppError> {
    let access = authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = require_tenant_id(access.tenant_id)?;

    let result = replay_provider_event(
        &state.db,
        BillingProviderEventReplayInput {
            tenant_id,
            actor_principal_id: access.auth.user_id,
            provider: request.provider,
            provider_event_id: request.provider_event_id,
            reason: request.reason,
        },
    )
    .await?;

    Ok(Json(result))
}

pub async fn create_provider_migration_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ProviderMigrationRequest>,
) -> Result<Json<BillingAdminMutationResult>, AppError> {
    let access = authorize_billing_admin_access(&state, &headers, workspace_id).await?;
    let tenant_id = require_tenant_id(access.tenant_id)?;

    let result = create_provider_migration_run(
        &state.db,
        BillingProviderMigrationInput {
            tenant_id,
            actor_principal_id: access.auth.user_id,
            from_provider: request.from_provider,
            to_provider: request.to_provider,
            reason: request.reason,
        },
    )
    .await?;

    Ok(Json(result))
}

pub(super) async fn authorize_billing_admin_access(
    state: &AppState,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> Result<WorkspaceAccess, AppError> {
    let access = authorize_workspace_action(
        &state.db,
        &state.redis,
        &state.jwt,
        headers,
        workspace_id,
        WorkspaceAction::ManageBilling,
        ResourceContext::default(),
    )
    .await?;
    verification::require_recent_step_up(&state.redis, &access.auth, Some(Aal::Aal2)).await?;
    Ok(access)
}

pub(super) fn require_tenant_id(tenant_id: Option<Uuid>) -> Result<Uuid, AppError> {
    tenant_id.ok_or_else(|| {
        AppError::bad_request(
            "tenant_context_required",
            "Billing admin actions require a tenant-scoped workspace.",
        )
    })
}

#[cfg(test)]
#[path = "identity.domains.billing.routes.admin.tests.rs"]
mod tests;
