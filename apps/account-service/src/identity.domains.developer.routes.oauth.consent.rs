use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        grpc, service,
        types::{DeveloperConsentScreenResponse, UpsertDeveloperConsentScreenInput},
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn get_consent_screen(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperConsentScreenResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;
    Ok(Json(
        grpc::get_consent_screen(tenant_id, auth.user_id, client_id).await?,
    ))
}

pub async fn upsert_consent_screen(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
    Json(input): Json<UpsertDeveloperConsentScreenInput>,
) -> Result<Json<DeveloperConsentScreenResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;
    Ok(Json(
        grpc::upsert_consent_screen(tenant_id, auth.user_id, client_id, input).await?,
    ))
}

pub(super) async fn ensure_oauth_client_in_tenant(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<(), AppError> {
    oauth_client_name_in_tenant(db, tenant_id, client_id)
        .await
        .map(|_| ())
}

pub(super) async fn oauth_client_name_in_tenant(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<String, AppError> {
    sqlx::query_scalar("SELECT name FROM oauth_clients WHERE tenant_id = $1 AND client_id = $2")
        .bind(tenant_id)
        .bind(client_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| {
            AppError::not_found(
                "oauth_client_not_found",
                "OAuth client not found in this tenant",
            )
        })
}
