use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        service,
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
    Ok(Json(
        get_consent_screen_response(&state.db, tenant_id, &client_id).await?,
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

    sqlx::query(
        r#"
        INSERT INTO developer_consent_screens (
          tenant_id,
          client_id,
          product_name,
          logo_url,
          support_url,
          privacy_url,
          terms_url,
          description,
          brand_color,
          custom_css,
          help_text,
          updated_by,
          updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now())
        ON CONFLICT (tenant_id, client_id) DO UPDATE
        SET product_name = EXCLUDED.product_name,
            logo_url = EXCLUDED.logo_url,
            support_url = EXCLUDED.support_url,
            privacy_url = EXCLUDED.privacy_url,
            terms_url = EXCLUDED.terms_url,
            description = EXCLUDED.description,
            brand_color = EXCLUDED.brand_color,
            custom_css = EXCLUDED.custom_css,
            help_text = EXCLUDED.help_text,
            updated_by = EXCLUDED.updated_by,
            updated_at = now()
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(input.product_name)
    .bind(input.logo_url)
    .bind(input.support_url)
    .bind(input.privacy_url)
    .bind(input.terms_url)
    .bind(input.description)
    .bind(input.brand_color)
    .bind(input.custom_css)
    .bind(input.help_text)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(
        get_consent_screen_response(&state.db, tenant_id, &client_id).await?,
    ))
}

pub(super) async fn ensure_oauth_client_in_tenant(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<(), AppError> {
    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT client_id FROM oauth_clients WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(db)
    .await?;

    if exists.is_none() {
        return Err(AppError::not_found(
            "oauth_client_not_found",
            "OAuth client not found in this tenant",
        ));
    }

    Ok(())
}

async fn get_consent_screen_response(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<DeveloperConsentScreenResponse, AppError> {
    ensure_oauth_client_in_tenant(db, tenant_id, client_id).await?;

    let screen = sqlx::query_as(
        r#"
        SELECT
          client_id,
          product_name,
          logo_url,
          support_url,
          privacy_url,
          terms_url,
          description,
          brand_color,
          custom_css,
          help_text,
          true AS configured,
          updated_at
        FROM developer_consent_screens
        WHERE tenant_id = $1
          AND client_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(db)
    .await?;

    Ok(screen.unwrap_or_else(|| DeveloperConsentScreenResponse {
        client_id: client_id.to_string(),
        product_name: String::new(),
        logo_url: None,
        support_url: None,
        privacy_url: None,
        terms_url: None,
        description: String::new(),
        brand_color: None,
        custom_css: None,
        help_text: None,
        configured: false,
        updated_at: None,
    }))
}
