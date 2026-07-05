use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use uuid::Uuid;

use crate::{
    app::BillingAppState,
    auth::{BillingWorkspacePermission, authorize_billing_workspace},
    http::error::AppError,
};

pub async fn get_billing_invoices(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<nvbes_billing::portal_views::BillingPortalInvoiceView>>, AppError> {
    authorize_read(&state, &headers, workspace_id).await?;
    Ok(Json(
        nvbes_billing::portal_views::fetch_portal_invoices(&state.db, workspace_id).await?,
    ))
}

pub async fn get_billing_cards(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<nvbes_billing::portal_views::BillingPortalPaymentMethodView>>, AppError> {
    authorize_read(&state, &headers, workspace_id).await?;
    Ok(Json(
        nvbes_billing::portal_views::fetch_portal_payment_methods(&state.db, workspace_id).await?,
    ))
}

pub async fn get_billing_subscriptions(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<nvbes_billing::portal_views::BillingPortalSubscriptionProviderView>>, AppError>
{
    authorize_read(&state, &headers, workspace_id).await?;
    Ok(Json(
        nvbes_billing::portal_views::fetch_portal_subscription_providers(&state.db, workspace_id)
            .await?,
    ))
}

async fn authorize_read(
    state: &BillingAppState,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    authorize_billing_workspace(
        &state.db,
        headers,
        workspace_id,
        BillingWorkspacePermission::Read,
    )
    .await?;
    Ok(())
}
