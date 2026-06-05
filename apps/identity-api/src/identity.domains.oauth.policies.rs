use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

use super::service::types::{
    CreateOAuthClientPolicyInput, DeleteOAuthClientPolicyResult, OAuthClientPoliciesResult,
    OAuthClientPolicyView, UpdateOAuthClientPolicyInput,
};
use super::{parse_client_policy_status, parse_step_up_level};

/// List client policies.
pub async fn list_client_policies(
    db: &PgPool,
    auth: &AuthContext,
    client_id: Option<String>,
) -> Result<OAuthClientPoliciesResult, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let (scope_type, scope_id) = super::logic::resolve_management_scope(auth)?;
    let rows = if let Some(client_id) = client_id {
        sqlx::query(
            r#"
            SELECT
              p.id,
              p.client_id,
              p.scope_type::text AS scope_type,
              p.scope_id,
              p.allowed_scopes,
              p.allowed_audiences,
              p.allowed_resources,
              p.required_acr::text AS required_acr,
              p.status::text AS status,
              p.created_at
            FROM oauth_client_policies p
            INNER JOIN oauth_clients c ON c.id = p.client_id
            WHERE c.client_id = $1
              AND c.tenant_id = $2
              AND (
                (c.owner_scope_type = $3::scope_type AND c.owner_scope_id = $4)
                OR (c.owner_scope_type = 'tenant' AND c.owner_scope_id = $2)
              )
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(client_id)
        .bind(auth.tenant_id)
        .bind(scope_type.as_str())
        .bind(scope_id)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT
              p.id,
              p.client_id,
              p.scope_type::text AS scope_type,
              p.scope_id,
              p.allowed_scopes,
              p.allowed_audiences,
              p.allowed_resources,
              p.required_acr::text AS required_acr,
              p.status::text AS status,
              p.created_at
            FROM oauth_client_policies p
            INNER JOIN oauth_clients c ON c.id = p.client_id
            WHERE c.tenant_id = $1
              AND (
                (c.owner_scope_type = $2::scope_type AND c.owner_scope_id = $3)
                OR (c.owner_scope_type = 'tenant' AND c.owner_scope_id = $1)
              )
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(auth.tenant_id)
        .bind(scope_type.as_str())
        .bind(scope_id)
        .fetch_all(db)
        .await?
    };

    Ok(OAuthClientPoliciesResult {
        policies: rows
            .into_iter()
            .map(|row| OAuthClientPolicyView {
                id: row.get("id"),
                client_id: row.get("client_id"),
                scope_type: row.get("scope_type"),
                scope_id: row.get("scope_id"),
                allowed_scopes: row.get("allowed_scopes"),
                allowed_audiences: row.get("allowed_audiences"),
                allowed_resources: row.get("allowed_resources"),
                required_acr: row.get("required_acr"),
                status: row.get("status"),
                created_at: row.get("created_at"),
            })
            .collect(),
    })
}

/// Create a new client policy.
pub async fn create_client_policy(
    db: &PgPool,
    auth: &AuthContext,
    client_id: String,
    input: CreateOAuthClientPolicyInput,
) -> Result<OAuthClientPolicyView, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let (scope_type, scope_id) =
        super::logic::resolve_policy_scope(auth, input.scope_type.as_deref(), input.scope_id)?;
    if input.allowed_scopes.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "At least one allowed scope is required.",
        ));
    }
    let required_acr = parse_step_up_level(input.required_acr.as_deref().unwrap_or("aal1"))?;
    let status = parse_client_policy_status(input.status.as_deref().unwrap_or("active"))?;
    let client_uuid =
        super::logic::client_uuid_by_client_id(db, auth.tenant_id, &client_id).await?;

    let row = sqlx::query(
        r#"
        INSERT INTO oauth_client_policies (
          client_id,
          scope_type,
          scope_id,
          allowed_scopes,
          allowed_audiences,
          allowed_resources,
          required_acr,
          status
        )
        VALUES ($1, $2::scope_type, $3, $4, $5, $6, $7::step_up_level, $8::client_policy_status)
        ON CONFLICT (client_id, scope_type, scope_id)
        DO UPDATE SET
          allowed_scopes = EXCLUDED.allowed_scopes,
          allowed_audiences = EXCLUDED.allowed_audiences,
          allowed_resources = EXCLUDED.allowed_resources,
          required_acr = EXCLUDED.required_acr,
          status = EXCLUDED.status
        RETURNING id, scope_type::text AS scope_type, scope_id, allowed_scopes, allowed_audiences, allowed_resources, required_acr::text AS required_acr, status::text AS status, created_at
        "#,
    )
    .bind(client_uuid)
    .bind(scope_type.as_str())
    .bind(scope_id)
    .bind(&input.allowed_scopes)
    .bind(&input.allowed_audiences)
    .bind(&input.allowed_resources)
    .bind(required_acr.as_str())
    .bind(status.as_str())
    .fetch_one(db)
    .await?;

    Ok(OAuthClientPolicyView {
        id: row.get("id"),
        client_id: client_uuid,
        scope_type: row.get("scope_type"),
        scope_id: row.get("scope_id"),
        allowed_scopes: row.get("allowed_scopes"),
        allowed_audiences: row.get("allowed_audiences"),
        allowed_resources: row.get("allowed_resources"),
        required_acr: row.get("required_acr"),
        status: row.get("status"),
        created_at: row.get("created_at"),
    })
}

/// Update a client policy.
pub async fn update_client_policy(
    db: &PgPool,
    auth: &AuthContext,
    policy_id: Uuid,
    input: UpdateOAuthClientPolicyInput,
) -> Result<OAuthClientPolicyView, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let required_acr = input
        .required_acr
        .as_deref()
        .map(parse_step_up_level)
        .transpose()?;
    let status = input
        .status
        .as_deref()
        .map(parse_client_policy_status)
        .transpose()?;

    let row = sqlx::query(
        r#"
        UPDATE oauth_client_policies p
        SET allowed_scopes = COALESCE($2::text[], allowed_scopes),
            allowed_audiences = COALESCE($3::text[], allowed_audiences),
            allowed_resources = COALESCE($4::text[], allowed_resources),
            required_acr = COALESCE($5::step_up_level, required_acr),
            status = COALESCE($6::client_policy_status, status)
        FROM oauth_clients c
        WHERE p.id = $1
          AND c.id = p.client_id
          AND c.tenant_id = $7
        RETURNING p.id, p.client_id, p.scope_type::text AS scope_type, p.scope_id, p.allowed_scopes, p.allowed_audiences, p.allowed_resources, p.required_acr::text AS required_acr, p.status::text AS status, p.created_at
        "#,
    )
    .bind(policy_id)
    .bind(input.allowed_scopes)
    .bind(input.allowed_audiences)
    .bind(input.allowed_resources)
    .bind(required_acr.as_ref().map(|value| value.as_str()))
    .bind(status.as_ref().map(|value| value.as_str()))
    .bind(auth.tenant_id)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Err(AppError::not_found(
            "client_policy_not_found",
            "The client policy was not found.",
        ));
    };

    Ok(OAuthClientPolicyView {
        id: row.get("id"),
        client_id: row.get("client_id"),
        scope_type: row.get("scope_type"),
        scope_id: row.get("scope_id"),
        allowed_scopes: row.get("allowed_scopes"),
        allowed_audiences: row.get("allowed_audiences"),
        allowed_resources: row.get("allowed_resources"),
        required_acr: row.get("required_acr"),
        status: row.get("status"),
        created_at: row.get("created_at"),
    })
}

/// Delete a client policy.
pub async fn delete_client_policy(
    db: &PgPool,
    auth: &AuthContext,
    policy_id: Uuid,
) -> Result<DeleteOAuthClientPolicyResult, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    let row = sqlx::query(
        r#"
        DELETE FROM oauth_client_policies p
        USING oauth_clients c
        WHERE p.id = $1
          AND c.id = p.client_id
          AND c.tenant_id = $2
        RETURNING p.id
        "#,
    )
    .bind(policy_id)
    .bind(auth.tenant_id)
    .fetch_optional(db)
    .await?;

    if row.is_none() {
        return Err(AppError::not_found(
            "client_policy_not_found",
            "The client policy was not found.",
        ));
    }

    Ok(DeleteOAuthClientPolicyResult { success: true })
}
