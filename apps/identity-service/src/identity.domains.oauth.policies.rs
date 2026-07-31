use nvbes_core::pagination::{KeysetCursor, decode_cursor, encode_cursor, page_from_rows};
use sqlx::{PgPool, Row, postgres::PgRow};
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
    limit: Option<i64>,
    cursor: Option<String>,
) -> Result<OAuthClientPoliciesResult, AppError> {
    let limit = limit.unwrap_or(50).clamp(1, 200) as usize;
    let cursor = cursor
        .as_deref()
        .map(decode_cursor::<KeysetCursor>)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let tenant_id = require_oauth_management_tenant(db, auth).await?;
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
              AND ($5::timestamp with time zone IS NULL OR (p.created_at, p.id) < ($5, $6))
            ORDER BY p.created_at DESC, p.id DESC
            LIMIT $7
            "#,
        )
        .bind(client_id)
        .bind(tenant_id)
        .bind(scope_type.as_str())
        .bind(scope_id)
        .bind(cursor.as_ref().map(|c| c.created_at))
        .bind(cursor.as_ref().map(|c| c.id))
        .bind((limit + 1) as i64)
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
              AND ($4::timestamp with time zone IS NULL OR (p.created_at, p.id) < ($4, $5))
            ORDER BY p.created_at DESC, p.id DESC
            LIMIT $6
            "#,
        )
        .bind(tenant_id)
        .bind(scope_type.as_str())
        .bind(scope_id)
        .bind(cursor.as_ref().map(|c| c.created_at))
        .bind(cursor.as_ref().map(|c| c.id))
        .bind((limit + 1) as i64)
        .fetch_all(db)
        .await?
    };

    let page = page_from_rows(rows, limit, |row| KeysetCursor {
        created_at: row.get("created_at"),
        id: row.get("id"),
    });
    let next_cursor = page
        .next_cursor
        .map(|c| encode_cursor(&c))
        .transpose()
        .map_err(|_| {
            AppError::internal("pagination_error", "Failed to encode pagination cursor.")
        })?;
    Ok(OAuthClientPoliciesResult {
        policies: page
            .items
            .into_iter()
            .map(|row| map_client_policy_row(&row))
            .collect(),
        next_cursor,
        has_more: page.has_more,
    })
}

/// Create a new client policy.
pub async fn create_client_policy(
    db: &PgPool,
    auth: &AuthContext,
    client_id: String,
    input: CreateOAuthClientPolicyInput,
) -> Result<OAuthClientPolicyView, AppError> {
    let tenant_id = require_oauth_management_tenant(db, auth).await?;
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
        super::logic::client_uuid_by_client_id(db, Some(tenant_id), &client_id).await?;

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

    Ok(map_client_policy_row_with_client_id(&row, client_uuid))
}

/// Update a client policy.
pub async fn update_client_policy(
    db: &PgPool,
    auth: &AuthContext,
    policy_id: Uuid,
    input: UpdateOAuthClientPolicyInput,
) -> Result<OAuthClientPolicyView, AppError> {
    let tenant_id = require_oauth_management_tenant(db, auth).await?;
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
    .bind(required_acr.as_deref())
    .bind(status.as_deref())
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Err(AppError::not_found(
            "client_policy_not_found",
            "The client policy was not found.",
        ));
    };

    Ok(map_client_policy_row(&row))
}

/// Delete a client policy.
pub async fn delete_client_policy(
    db: &PgPool,
    auth: &AuthContext,
    policy_id: Uuid,
) -> Result<DeleteOAuthClientPolicyResult, AppError> {
    let tenant_id = require_oauth_management_tenant(db, auth).await?;
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
    .bind(tenant_id)
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

async fn require_oauth_management_tenant(
    db: &PgPool,
    auth: &AuthContext,
) -> Result<Uuid, AppError> {
    let tenant_id = auth.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before using OAuth management.",
        )
    })?;
    crate::domains::authz::ensure_tenant_management_access(db, auth, tenant_id).await?;
    Ok(tenant_id)
}

fn map_client_policy_row(row: &PgRow) -> OAuthClientPolicyView {
    OAuthClientPolicyView {
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
    }
}

fn map_client_policy_row_with_client_id(row: &PgRow, client_id: Uuid) -> OAuthClientPolicyView {
    OAuthClientPolicyView {
        client_id,
        ..map_client_policy_row(row)
    }
}
