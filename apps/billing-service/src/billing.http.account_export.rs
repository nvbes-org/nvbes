use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use chrono::Utc;
use nvbes_product_account::export_event::{
    AccountExportContractError, AccountExportFragmentV1, AccountExportRequestedV1,
};
use serde_json::Value;

use crate::{app::BillingAppState, http::error::AppError};

pub fn router() -> Router<BillingAppState> {
    Router::new().route("/internal/v1/account-exports", post(export_account))
}

async fn export_account(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Json(command): Json<AccountExportRequestedV1>,
) -> Result<Json<AccountExportFragmentV1>, AppError> {
    if !nvbes_core::http::internal_service::bearer_matches(&headers, &state.internal_service_token)
    {
        return Err(AppError::unauthorized(
            "invalid_internal_token",
            "A valid Billing internal token is required.",
        ));
    }
    validate_command(&command)?;
    let data = build_fragment(&state.db, command.principal_id).await?;
    Ok(Json(AccountExportFragmentV1::new(
        "billing",
        command.principal_id,
        data,
    )))
}

fn validate_command(command: &AccountExportRequestedV1) -> Result<(), AppError> {
    match command.validate(Utc::now()) {
        Ok(()) => Ok(()),
        Err(AccountExportContractError::UnsupportedVersion) => Err(AppError::bad_request(
            "unsupported_account_export_event",
            "The Account export event version is not supported.",
        )),
        Err(_) => Err(AppError::bad_request(
            "invalid_account_export_event",
            "The Account export event is invalid.",
        )),
    }
}

async fn build_fragment(db: &sqlx::PgPool, principal_id: uuid::Uuid) -> Result<Value, AppError> {
    sqlx::query_scalar(
        r#"WITH owned_workspaces AS (
          SELECT id, tenant_id FROM workspaces
          WHERE owner_user_id = $1 AND deleted_at IS NULL
        ), owned_accounts AS (
          SELECT id FROM billing_accounts WHERE workspace_id IN (SELECT id FROM owned_workspaces)
        ), owned_tenants AS (
          SELECT DISTINCT tenant_id FROM owned_workspaces
        )
        SELECT jsonb_build_object(
          'principal_projection', (SELECT jsonb_build_object(
            'id', id, 'display_name', display_name, 'status', status,
            'created_at', created_at, 'updated_at', updated_at)
            FROM principals WHERE id = $1),
          'user_projection', (SELECT to_jsonb(u) FROM users u WHERE principal_id = $1),
          'workspace_memberships', COALESCE((SELECT jsonb_agg(
            jsonb_build_object('workspace_id', m.workspace_id, 'workspace_name', w.name,
              'role', m.role, 'status', m.status, 'created_at', m.created_at,
              'updated_at', m.updated_at) ORDER BY m.created_at DESC)
            FROM workspace_memberships m JOIN workspaces w ON w.id = m.workspace_id
            WHERE m.principal_id = $1), '[]'::jsonb),
          'owned_workspaces', COALESCE((SELECT jsonb_agg(to_jsonb(w) ORDER BY created_at DESC)
            FROM workspaces w WHERE id IN (SELECT id FROM owned_workspaces)), '[]'::jsonb),
          'billing_accounts', COALESCE((SELECT jsonb_agg(to_jsonb(a) ORDER BY created_at DESC)
            FROM billing_accounts a WHERE id IN (SELECT id FROM owned_accounts)), '[]'::jsonb),
          'customer_profiles', COALESCE((SELECT jsonb_agg(to_jsonb(p) ORDER BY created_at DESC)
            FROM billing_customer_profiles p WHERE billing_account_id IN (SELECT id FROM owned_accounts)), '[]'::jsonb),
          'tax_profiles', COALESCE((SELECT jsonb_agg(to_jsonb(p) ORDER BY created_at DESC)
            FROM billing_tax_profiles p WHERE billing_account_id IN (SELECT id FROM owned_accounts)), '[]'::jsonb),
          'subscriptions', COALESCE((SELECT jsonb_agg(to_jsonb(s) ORDER BY created_at DESC)
            FROM subscriptions s WHERE workspace_id IN (SELECT id FROM owned_workspaces)), '[]'::jsonb),
          'invoices', COALESCE((SELECT jsonb_agg(to_jsonb(i) ORDER BY created_at DESC)
            FROM billing_invoices i WHERE tenant_id IN (SELECT tenant_id FROM owned_tenants)), '[]'::jsonb),
          'payments', COALESCE((SELECT jsonb_agg(to_jsonb(p) ORDER BY created_at DESC)
            FROM billing_payments p WHERE tenant_id IN (SELECT tenant_id FROM owned_tenants)), '[]'::jsonb),
          'refunds', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY created_at DESC)
            FROM billing_refunds r WHERE tenant_id IN (SELECT tenant_id FROM owned_tenants)), '[]'::jsonb),
          'audit_events', COALESCE((SELECT jsonb_agg(to_jsonb(a) ORDER BY created_at DESC)
            FROM audit_events a WHERE actor_principal_id = $1), '[]'::jsonb)
        )"#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    #[ignore = "requires NVBES_BILLING_TEST_DATABASE_URL pointing to disposable PostgreSQL"]
    async fn fragment_query_is_valid_on_the_billing_schema() {
        let url = std::env::var("NVBES_BILLING_TEST_DATABASE_URL").unwrap();
        let db = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let fragment = super::build_fragment(&db, uuid::Uuid::new_v4())
            .await
            .unwrap();
        assert!(fragment.is_object());
        assert!(!fragment.to_string().contains("provider_secret"));
    }
}
