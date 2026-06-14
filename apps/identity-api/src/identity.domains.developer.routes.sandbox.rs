use axum::{Extension, Json, extract::State};
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        rbac::DeveloperPermission,
        service,
        types::{
            DeveloperSandboxResponse, DeveloperSandboxTenantSummary, UpsertDeveloperSandboxInput,
        },
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn get_sandbox(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperSandboxResponse>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    Ok(Json(DeveloperSandboxResponse {
        sandbox: find_sandbox(&state.db, tenant_id).await?,
    }))
}

pub async fn upsert_sandbox(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<UpsertDeveloperSandboxInput>,
) -> Result<Json<DeveloperSandboxTenantSummary>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    let data_profile = input.data_profile.unwrap_or_else(|| "minimal".to_string());
    validate_data_profile(&data_profile)?;

    if let Some(existing) = find_sandbox(&state.db, tenant_id).await? {
        let sandbox = sqlx::query_as(
            r#"
            UPDATE developer_sandbox_tenants
            SET data_profile = $2,
                status = 'active',
                updated_at = now()
            WHERE tenant_id = $1
            RETURNING
              tenant_id,
              sandbox_tenant_id,
              (SELECT name FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_name,
              (SELECT slug FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_slug,
              status,
              data_profile,
              reset_requested_at,
              updated_at
            "#,
        )
        .bind(existing.tenant_id)
        .bind(&data_profile)
        .fetch_one(&state.db)
        .await?;
        return Ok(Json(sandbox));
    }

    let sandbox_tenant_id = Uuid::new_v4();
    let slug = format!(
        "sandbox-{}-{}",
        tenant_id.simple(),
        sandbox_tenant_id.simple()
    );
    let name = "Developer Sandbox".to_string();
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier)
        VALUES ($1, 'organization', $2, $3, 'active', 'sandbox')
        "#,
    )
    .bind(sandbox_tenant_id)
    .bind(&name)
    .bind(&slug)
    .execute(&state.db)
    .await?;

    let sandbox = sqlx::query_as(
        r#"
        INSERT INTO developer_sandbox_tenants (
          tenant_id,
          sandbox_tenant_id,
          status,
          data_profile
        )
        VALUES ($1, $2, 'active', $3)
        RETURNING
          tenant_id,
          sandbox_tenant_id,
          $4::text AS sandbox_name,
          $5::text AS sandbox_slug,
          status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(sandbox_tenant_id)
    .bind(&data_profile)
    .bind(&name)
    .bind(&slug)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(sandbox))
}

pub async fn reset_sandbox(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperSandboxTenantSummary>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::SandboxUse).await?;
    let sandbox = sqlx::query_as(
        r#"
        UPDATE developer_sandbox_tenants
        SET status = 'resetting',
            reset_requested_at = now(),
            updated_at = now()
        WHERE tenant_id = $1
        RETURNING
          tenant_id,
          sandbox_tenant_id,
          (SELECT name FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_name,
          (SELECT slug FROM tenants WHERE id = sandbox_tenant_id) AS sandbox_slug,
          status,
          data_profile,
          reset_requested_at,
          updated_at
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("sandbox_not_found", "Sandbox tenant not found."))?;

    Ok(Json(sandbox))
}

async fn find_sandbox(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Option<DeveloperSandboxTenantSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          s.tenant_id,
          s.sandbox_tenant_id,
          t.name AS sandbox_name,
          t.slug AS sandbox_slug,
          s.status,
          s.data_profile,
          s.reset_requested_at,
          s.updated_at
        FROM developer_sandbox_tenants s
        INNER JOIN tenants t ON t.id = s.sandbox_tenant_id
        WHERE s.tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

fn validate_data_profile(data_profile: &str) -> Result<(), AppError> {
    match data_profile {
        "minimal" | "oauth" | "full" => Ok(()),
        _ => Err(AppError::bad_request(
            "invalid_sandbox_data_profile",
            "Sandbox data profile must be minimal, oauth, or full.",
        )),
    }
}
