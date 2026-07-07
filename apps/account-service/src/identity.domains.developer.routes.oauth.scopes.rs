use axum::{
    Extension, Json,
    extract::{Path, State},
};

use crate::{
    app::AppState,
    domains::developer::{
        grpc,
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
    State(_state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperScopeRegistryResponse>, AppError> {
    Ok(Json(grpc::list_scopes(auth.user_id).await?))
}

pub async fn create_scope(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(input): Json<CreateScopeInput>,
) -> Result<Json<DeveloperScopeRegistryEntry>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage)
            .await?;
    let input = normalize_create_scope_input(input)?;
    let scope = grpc::create_scope(tenant_id, auth.user_id, input).await?;
    upsert_scope_metadata(&state.db, &scope.scope_key, &scope.description, &scope.risk).await?;

    Ok(Json(scope))
}

pub async fn update_scope(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(scope_key): Path<String>,
    Json(input): Json<UpdateScopeInput>,
) -> Result<Json<DeveloperScopeRegistryEntry>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage)
            .await?;
    let input = normalize_update_scope_input(input)?;
    let scope = grpc::update_scope(tenant_id, auth.user_id, scope_key, input).await?;
    upsert_scope_metadata(&state.db, &scope.scope_key, &scope.description, &scope.risk).await?;

    Ok(Json(scope))
}

pub async fn delete_scope(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(scope_key): Path<String>,
) -> Result<Json<()>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage)
            .await?;
    grpc::delete_scope(tenant_id, auth.user_id, scope_key.clone()).await?;

    sqlx::query("DELETE FROM oauth_scope_metadata WHERE scope = $1")
        .bind(&scope_key)
        .execute(&state.db)
        .await?;

    Ok(Json(()))
}

fn normalize_create_scope_input(input: CreateScopeInput) -> Result<CreateScopeInput, AppError> {
    let scope_key = input.scope_key.trim().to_string();
    if scope_key.is_empty() {
        return Err(AppError::bad_request(
            "invalid_scope_key",
            "Scope key cannot be empty.",
        ));
    }

    let risk = validate_risk(&input.risk)?;
    let lifecycle = validate_lifecycle(input.lifecycle.as_deref().unwrap_or("proposed"))?;
    Ok(CreateScopeInput {
        scope_key,
        display_name: input.display_name,
        description: input.description,
        risk,
        owner_team: input.owner_team,
        lifecycle: Some(lifecycle),
        allowed_audiences: input.allowed_audiences,
    })
}

fn normalize_update_scope_input(input: UpdateScopeInput) -> Result<UpdateScopeInput, AppError> {
    let risk = validate_risk(&input.risk)?;
    let lifecycle = validate_lifecycle(&input.lifecycle)?;

    Ok(UpdateScopeInput {
        display_name: input.display_name,
        description: input.description,
        risk,
        owner_team: input.owner_team,
        lifecycle,
        allowed_audiences: input.allowed_audiences,
    })
}

async fn upsert_scope_metadata(
    db: &sqlx::PgPool,
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
    .execute(db)
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
