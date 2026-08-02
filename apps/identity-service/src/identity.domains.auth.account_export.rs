use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use chrono::Utc;
use nvbes_product_account::export_event::{
    AccountExportContractError, AccountExportFragmentV1, AccountExportRequestedV1,
};
use serde_json::Value;

use crate::{app::AppState, http::error::AppError};

pub fn router() -> Router<AppState> {
    Router::new().route("/account-exports", post(export_account))
}

async fn export_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<AccountExportRequestedV1>,
) -> Result<Json<AccountExportFragmentV1>, AppError> {
    if !nvbes_core::http::internal_service::bearer_matches(&headers, &state.identity_internal_token)
    {
        return Err(AppError::unauthorized(
            "invalid_internal_token",
            "A valid Identity internal token is required.",
        ));
    }
    validate_command(&command)?;
    let data = build_fragment(&state.db, command.principal_id).await?;
    Ok(Json(AccountExportFragmentV1::new(
        "identity",
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
        r#"SELECT jsonb_build_object(
          'principal', (SELECT jsonb_build_object(
            'id', id, 'tenant_id', tenant_id, 'principal_kind', principal_kind,
            'status', status, 'display_name', display_name,
            'created_at', created_at, 'updated_at', updated_at)
            FROM principals WHERE id = $1),
          'authentication_account', (SELECT jsonb_build_object(
            'email', email, 'email_verified_at', email_verified_at, 'status', status,
            'created_at', created_at, 'updated_at', updated_at,
            'password_last_changed_at', password_last_changed_at)
            FROM users WHERE principal_id = $1),
          'email_addresses', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'id', id, 'email', email, 'is_primary', is_primary, 'verified_at', verified_at,
            'created_at', created_at, 'updated_at', updated_at, 'deleted_at', deleted_at)
            ORDER BY created_at DESC) FROM user_email_addresses WHERE principal_id = $1), '[]'::jsonb),
          'linked_identities', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'id', id, 'provider_type', provider_type, 'provider_id', provider_id,
            'subject', subject, 'email', email, 'email_verified', email_verified,
            'created_at', created_at) ORDER BY created_at DESC)
            FROM user_identities WHERE principal_id = $1), '[]'::jsonb),
          'mfa_factors', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'id', id, 'factor_type', factor_type, 'status', status, 'label', label,
            'created_at', created_at, 'confirmed_at', confirmed_at, 'last_used_at', last_used_at)
            ORDER BY created_at DESC) FROM mfa_factors WHERE principal_id = $1), '[]'::jsonb),
          'sessions', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'session_id', session_id, 'tenant_id', tenant_id, 'organization_id', organization_id,
            'workspace_id', workspace_id, 'client_id', client_id, 'acr', acr, 'amr', amr,
            'auth_time', auth_time, 'expires_at', expires_at, 'revoked_at', revoked_at,
            'ip', ip, 'user_agent', user_agent, 'account_device_id', account_device_id,
            'device_trust_level', device_trust_level, 'risk_score', risk_score,
            'risk_decision', risk_decision, 'last_seen_at', last_seen_at, 'created_at', created_at)
            ORDER BY created_at DESC) FROM user_sessions WHERE principal_id = $1), '[]'::jsonb),
          'devices', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'id', id, 'display_name', display_name, 'trust_level', trust_level,
            'trust_score', trust_score, 'successful_auth_count', successful_auth_count,
            'failed_auth_count', failed_auth_count, 'last_country_code', last_country_code,
            'last_user_agent_family', last_user_agent_family, 'first_seen_at', first_seen_at,
            'last_seen_at', last_seen_at, 'last_verified_at', last_verified_at,
            'trusted_at', trusted_at, 'revoked_at', revoked_at, 'created_at', created_at,
            'updated_at', updated_at) ORDER BY created_at DESC)
            FROM account_devices WHERE principal_id = $1), '[]'::jsonb),
          'oauth_consents', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'id', c.id, 'client_id', client.client_id, 'client_name', client.name,
            'scope', c.scope, 'audience', c.audience, 'resource_indicators', c.resource_indicators,
            'granted_at', c.granted_at, 'expires_at', c.expires_at, 'revoked_at', c.revoked_at)
            ORDER BY c.granted_at DESC) FROM oauth_consents c
            JOIN oauth_clients client ON client.id = c.client_id WHERE c.principal_id = $1), '[]'::jsonb),
          'tenant_memberships', COALESCE((SELECT jsonb_agg(to_jsonb(m) ORDER BY created_at DESC)
            FROM tenant_memberships m WHERE principal_id = $1), '[]'::jsonb),
          'organization_memberships', COALESCE((SELECT jsonb_agg(to_jsonb(m) ORDER BY created_at DESC)
            FROM organization_memberships m WHERE principal_id = $1), '[]'::jsonb),
          'workspace_memberships', COALESCE((SELECT jsonb_agg(to_jsonb(m) ORDER BY created_at DESC)
            FROM workspace_memberships m WHERE principal_id = $1), '[]'::jsonb),
          'developer_roles', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY created_at DESC)
            FROM developer_role_assignments r WHERE principal_id = $1), '[]'::jsonb),
          'privileged_access', COALESCE((SELECT jsonb_agg(to_jsonb(g) ORDER BY created_at DESC)
            FROM privileged_access_grants g WHERE principal_id = $1), '[]'::jsonb),
          'risk_events', COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY created_at DESC)
            FROM risk_events r WHERE principal_id = $1), '[]'::jsonb),
          'audit_events', COALESCE((SELECT jsonb_agg(jsonb_build_object(
            'id', id, 'tenant_id', tenant_id, 'workspace_id', workspace_id, 'action', action,
            'target_type', target_type, 'target_id', target_id, 'ip', ip,
            'user_agent', user_agent, 'created_at', created_at) ORDER BY created_at DESC)
            FROM audit_events WHERE actor_principal_id = $1), '[]'::jsonb)
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
    #[ignore = "requires NVBES_IDENTITY_TEST_DATABASE_URL pointing to disposable PostgreSQL"]
    async fn fragment_query_excludes_authentication_secrets() {
        let url = std::env::var("NVBES_IDENTITY_TEST_DATABASE_URL").unwrap();
        let db = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let fragment = super::build_fragment(&db, uuid::Uuid::new_v4())
            .await
            .unwrap();
        let encoded = fragment.to_string();
        for secret in [
            "password_hash",
            "totp_secret",
            "browser_session_token_hash",
            "installation_token_hash",
        ] {
            assert!(!encoded.contains(secret));
        }
    }
}
