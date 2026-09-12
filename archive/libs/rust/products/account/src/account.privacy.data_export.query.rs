//! Account-owned privacy export projection.

use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{AccountError, AccountResult};

use super::DATA_EXPORT_TTL_SECONDS;

pub async fn build_account_export(
    db: &PgPool,
    principal_id: Uuid,
    email_activity: JsonValue,
) -> AccountResult<JsonValue> {
    let mut export = sqlx::query_scalar::<_, JsonValue>(
        r#"
        SELECT jsonb_build_object(
          'exported_at', NOW(),
          'expires_in_seconds', $2,
          'scope', 'identity.account',
          'principal_id', $1,
          'principal', (
            SELECT jsonb_build_object(
              'id', id,
              'tenant_id', tenant_id,
              'principal_kind', principal_kind::text,
              'status', status::text,
              'display_name', display_name,
              'created_at', created_at,
              'updated_at', updated_at
            )
            FROM principals
            WHERE id = $1
          ),
          'user', (
            SELECT jsonb_build_object(
              'principal_id', principal_id,
              'email', email,
              'name', name,
              'firstname', firstname,
              'lastname', lastname,
              'username', username,
              'birthdate', birthdate,
              'region', region,
              'email_verified_at', email_verified_at,
              'status', status::text,
              'preferences', preferences,
              'notifications', notifications,
              'password_last_changed_at', password_last_changed_at,
              'created_at', created_at,
              'updated_at', updated_at
            )
            FROM users
            WHERE principal_id = $1
          ),
          'memberships', jsonb_build_object(
            'tenants', COALESCE((
              SELECT jsonb_agg(to_jsonb(tm) || jsonb_build_object('tenant_name', t.name, 'tenant_kind', t.kind::text) ORDER BY tm.created_at DESC)
              FROM tenant_memberships tm
              INNER JOIN tenants t ON t.id = tm.tenant_id
              WHERE tm.principal_id = $1
            ), '[]'::jsonb),
            'organizations', COALESCE((
              SELECT jsonb_agg(to_jsonb(om) || jsonb_build_object('organization_name', o.name) ORDER BY om.created_at DESC)
              FROM organization_memberships om
              INNER JOIN organizations o ON o.id = om.organization_id
              WHERE om.principal_id = $1
            ), '[]'::jsonb),
            'workspaces', COALESCE((
              SELECT jsonb_agg(to_jsonb(wm) || jsonb_build_object('workspace_name', w.name, 'workspace_type', w.workspace_type::text) ORDER BY wm.created_at DESC)
              FROM workspace_memberships wm
              INNER JOIN workspaces w ON w.id = wm.workspace_id
              WHERE wm.principal_id = $1
            ), '[]'::jsonb)
          ),
          'federated_identities', COALESCE((
            SELECT jsonb_agg(to_jsonb(user_identities) ORDER BY created_at DESC)
            FROM user_identities
            WHERE principal_id = $1
          ), '[]'::jsonb),
          'devices', COALESCE((
            SELECT jsonb_agg(to_jsonb(devices) ORDER BY created_at DESC)
            FROM devices
            WHERE principal_id = $1
          ), '[]'::jsonb),
          'mfa_factors', COALESCE((
            SELECT jsonb_agg(
              (to_jsonb(mfa_factors) - 'totp_secret_base32' - 'factor_data')
              || jsonb_build_object('factor_data_present', factor_data <> '{}'::jsonb)
              ORDER BY created_at DESC
            )
            FROM mfa_factors
            WHERE principal_id = $1
          ), '[]'::jsonb),
          'oauth_consents', COALESCE((
            SELECT jsonb_agg(to_jsonb(oauth_consents) ORDER BY granted_at DESC)
            FROM oauth_consents
            WHERE principal_id = $1
          ), '[]'::jsonb),
          'owned_oauth_clients', COALESCE((
            SELECT jsonb_agg(
              to_jsonb(oauth_clients)
              - 'client_secret_hash'
              - 'client_assertion_public_key_jwk'
              || jsonb_build_object('client_assertion_public_key_configured', client_assertion_public_key_jwk IS NOT NULL)
              ORDER BY created_at DESC
            )
            FROM oauth_clients
            WHERE owner_scope_id = $1
          ), '[]'::jsonb),
          'created_service_accounts', COALESCE((
            SELECT jsonb_agg((to_jsonb(service_accounts) - 'secret_hash') ORDER BY created_at DESC)
            FROM service_accounts
            WHERE created_by_principal_id = $1
          ), '[]'::jsonb),
          'user_consents', COALESCE((
            SELECT jsonb_agg(to_jsonb(user_consents) ORDER BY granted_at DESC)
            FROM user_consents
            WHERE principal_id = $1
          ), '[]'::jsonb),
          'risk_events', COALESCE((
            SELECT jsonb_agg(to_jsonb(risk_events) ORDER BY created_at DESC)
            FROM risk_events
            WHERE principal_id = $1
          ), '[]'::jsonb),
          'audit_events', COALESCE((
            SELECT jsonb_agg(to_jsonb(audit_events) ORDER BY created_at DESC)
            FROM audit_events
            WHERE actor_principal_id = $1
          ), '[]'::jsonb),
          'enterprise_password_recovery_requests', COALESCE((
            SELECT jsonb_agg(
              (
                to_jsonb(enterprise_password_recovery_requests)
                - 'reset_token_hash'
                - 'approved_by_principal_id'
                - 'secondary_approved_by_principal_id'
                - 'rejected_by_principal_id'
                - 'review_reason'
              )
              ORDER BY created_at DESC
            )
            FROM enterprise_password_recovery_requests
            WHERE principal_id = $1
          ), '[]'::jsonb)
        )
        "#,
    )
    .bind(principal_id)
    .bind(i64::try_from(DATA_EXPORT_TTL_SECONDS).unwrap_or(86_400))
    .fetch_one(db)
    .await
    .map_err(AccountError::from)?;

    let activity = email_activity.as_object().ok_or_else(|| {
        AccountError::internal(
            "data_export_email_activity_invalid",
            "Email activity projection must be a JSON object.",
        )
    })?;
    let export_object = export.as_object_mut().ok_or_else(|| {
        AccountError::internal(
            "data_export_projection_invalid",
            "Account export projection must be a JSON object.",
        )
    })?;
    export_object.insert(
        "email_messages".to_string(),
        activity
            .get("messages")
            .cloned()
            .unwrap_or_else(|| JsonValue::Array(Vec::new())),
    );
    export_object.insert(
        "email_events".to_string(),
        activity
            .get("provider_events")
            .cloned()
            .unwrap_or_else(|| JsonValue::Array(Vec::new())),
    );

    Ok(export)
}
