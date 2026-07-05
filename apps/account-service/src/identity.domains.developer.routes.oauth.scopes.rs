use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sqlx::PgPool;

use crate::{
    app::AppState,
    domains::developer::{
        rbac::DeveloperPermission,
        service,
        types::{
            CreateScopeInput, DeveloperScopeRegistryEntry, DeveloperScopeRegistryResponse,
            UpdateScopeInput,
        },
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_scopes(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<DeveloperScopeRegistryResponse>, AppError> {
    let scopes = list_scope_registry_entries(&state.db).await?;
    Ok(Json(DeveloperScopeRegistryResponse { scopes }))
}

pub async fn create_scope(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<CreateScopeInput>,
) -> Result<Json<DeveloperScopeRegistryEntry>, AppError> {
    service::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage).await?;

    let scope_key = input.scope_key.trim();
    if scope_key.is_empty() {
        return Err(AppError::bad_request(
            "invalid_scope_key",
            "Scope key cannot be empty.",
        ));
    }

    let risk = validate_risk(&input.risk)?;
    let lifecycle = validate_lifecycle(input.lifecycle.as_deref().unwrap_or("proposed"))?;
    let mut tx = state.db.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO developer_scope_registry (
            scope_key, display_name, description, risk, owner_team, lifecycle, allowed_audiences
        )
        VALUES ($1, $2, $3, $4::developer_scope_risk, $5, $6::developer_scope_lifecycle, $7)
        "#,
    )
    .bind(scope_key)
    .bind(&input.display_name)
    .bind(&input.description)
    .bind(&risk)
    .bind(&input.owner_team)
    .bind(&lifecycle)
    .bind(&input.allowed_audiences)
    .execute(&mut *tx)
    .await?;

    upsert_scope_metadata(&mut tx, scope_key, &input.description, &risk).await?;
    tx.commit().await?;

    Ok(Json(DeveloperScopeRegistryEntry {
        scope_key: scope_key.to_string(),
        display_name: input.display_name,
        description: input.description,
        risk,
        owner_team: input.owner_team,
        lifecycle,
        allowed_audiences: input.allowed_audiences,
    }))
}

pub async fn update_scope(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(scope_key): Path<String>,
    Json(input): Json<UpdateScopeInput>,
) -> Result<Json<DeveloperScopeRegistryEntry>, AppError> {
    service::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage).await?;

    let risk = validate_risk(&input.risk)?;
    let lifecycle = validate_lifecycle(&input.lifecycle)?;
    ensure_scope_exists(&state.db, &scope_key).await?;
    let mut tx = state.db.begin().await?;

    sqlx::query(
        r#"
        UPDATE developer_scope_registry
        SET display_name = $1,
            description = $2,
            risk = $3::developer_scope_risk,
            owner_team = $4,
            lifecycle = $5::developer_scope_lifecycle,
            allowed_audiences = $6,
            updated_at = now()
        WHERE scope_key = $7
        "#,
    )
    .bind(&input.display_name)
    .bind(&input.description)
    .bind(&risk)
    .bind(&input.owner_team)
    .bind(&lifecycle)
    .bind(&input.allowed_audiences)
    .bind(&scope_key)
    .execute(&mut *tx)
    .await?;

    upsert_scope_metadata(&mut tx, &scope_key, &input.description, &risk).await?;
    tx.commit().await?;

    Ok(Json(DeveloperScopeRegistryEntry {
        scope_key,
        display_name: input.display_name,
        description: input.description,
        risk,
        owner_team: input.owner_team,
        lifecycle,
        allowed_audiences: input.allowed_audiences,
    }))
}

pub async fn delete_scope(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(scope_key): Path<String>,
) -> Result<Json<()>, AppError> {
    service::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage).await?;
    ensure_scope_exists(&state.db, &scope_key).await?;

    let mut tx = state.db.begin().await?;

    sqlx::query("DELETE FROM developer_scope_registry WHERE scope_key = $1")
        .bind(&scope_key)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM oauth_scope_metadata WHERE scope = $1")
        .bind(&scope_key)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(()))
}

async fn list_scope_registry_entries(
    db: &PgPool,
) -> Result<Vec<DeveloperScopeRegistryEntry>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          scope_key,
          display_name,
          description,
          risk::text AS risk,
          owner_team,
          lifecycle::text AS lifecycle,
          allowed_audiences
        FROM developer_scope_registry
        ORDER BY scope_key ASC
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

async fn ensure_scope_exists(db: &PgPool, scope_key: &str) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM developer_scope_registry WHERE scope_key = $1)",
    )
    .bind(scope_key)
    .fetch_one(db)
    .await?;

    if !exists {
        return Err(AppError::not_found(
            "scope_not_found",
            "The requested scope was not found.",
        ));
    }

    Ok(())
}

async fn upsert_scope_metadata(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    scope_key: &str,
    description: &str,
    risk: &str,
) -> Result<(), AppError> {
    let requires_admin_consent = matches!(risk, "high" | "restricted");
    sqlx::query(
        r#"
        INSERT INTO oauth_scope_metadata (scope, description, requires_admin_consent)
        VALUES ($1, $2, $3)
        ON CONFLICT (scope) DO UPDATE
        SET description = EXCLUDED.description,
            requires_admin_consent = EXCLUDED.requires_admin_consent
        "#,
    )
    .bind(scope_key)
    .bind(description)
    .bind(requires_admin_consent)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn validate_risk(value: &str) -> Result<String, AppError> {
    let risk = value.to_lowercase();
    if !matches!(risk.as_str(), "low" | "medium" | "high" | "restricted") {
        return Err(AppError::bad_request("invalid_risk", "Invalid risk level."));
    }
    Ok(risk)
}

fn validate_lifecycle(value: &str) -> Result<String, AppError> {
    let lifecycle = value.to_lowercase();
    if !matches!(
        lifecycle.as_str(),
        "proposed" | "active" | "deprecated" | "retired"
    ) {
        return Err(AppError::bad_request(
            "invalid_lifecycle",
            "Invalid lifecycle status.",
        ));
    }
    Ok(lifecycle)
}
