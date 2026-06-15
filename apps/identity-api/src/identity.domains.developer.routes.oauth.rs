use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        rbac::DeveloperPermission,
        service,
        types::{
            DeveloperConsentScreenResponse, DeveloperMarketplaceAppSummary,
            DeveloperMarketplaceAppsResponse, DeveloperOAuthClientSummary,
            DeveloperOAuthClientsResponse, DeveloperScopeRegistryEntry,
            DeveloperScopeRegistryResponse, ReviewMarketplaceAppInput,
            UpsertDeveloperConsentScreenInput,
        },
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_oauth_clients(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperOAuthClientsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let oauth_clients = list_oauth_client_summaries(&state.db, tenant_id).await?;
    Ok(Json(DeveloperOAuthClientsResponse { oauth_clients }))
}

pub async fn list_marketplace_apps(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperMarketplaceAppsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let apps = list_marketplace_app_summaries(&state.db, tenant_id).await?;
    Ok(Json(DeveloperMarketplaceAppsResponse { apps }))
}

pub async fn list_scopes(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<DeveloperScopeRegistryResponse>, AppError> {
    let scopes = list_scope_registry_entries(&state.db).await?;
    Ok(Json(DeveloperScopeRegistryResponse { scopes }))
}

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

async fn list_oauth_client_summaries(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperOAuthClientSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          c.client_id,
          c.name,
          CASE WHEN c.revoked_at IS NULL THEN 'active' ELSE 'revoked' END AS status,
          m.status::text AS marketplace_status,
          cs.client_id IS NOT NULL AS consent_screen_configured,
          cardinality(c.redirect_uris)::bigint AS redirect_uri_count,
          COALESCE(policy.allowed_scopes, '{}'::text[]) AS allowed_scopes,
          COALESCE(health.status, 'unknown') AS health_status
        FROM oauth_clients c
        LEFT JOIN developer_marketplace_apps m
          ON m.tenant_id = c.tenant_id
         AND m.client_id = c.client_id
        LEFT JOIN developer_consent_screens cs
          ON cs.tenant_id = c.tenant_id
         AND cs.client_id = c.client_id
        LEFT JOIN LATERAL (
          SELECT array_agg(DISTINCT scope ORDER BY scope) AS allowed_scopes
          FROM oauth_client_policies p
          CROSS JOIN LATERAL unnest(p.allowed_scopes) AS scope
          WHERE p.client_id = c.id
            AND p.status = 'active'
        ) policy ON true
        LEFT JOIN LATERAL (
          SELECT h.status::text AS status
          FROM developer_health_checks h
          WHERE h.tenant_id = c.tenant_id
            AND h.target_type = 'oauth_client'
            AND h.target_id = c.client_id
          ORDER BY h.checked_at DESC
          LIMIT 1
        ) health ON true
        WHERE c.tenant_id = $1
        ORDER BY c.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

async fn list_marketplace_app_summaries(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<DeveloperMarketplaceAppSummary>, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          m.client_id,
          c.name,
          m.status::text AS status,
          m.review_reason,
          m.created_at,
          m.updated_at
        FROM developer_marketplace_apps m
        INNER JOIN oauth_clients c
          ON c.tenant_id = m.tenant_id
         AND c.client_id = m.client_id
        WHERE m.tenant_id = $1
        ORDER BY m.updated_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
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

async fn ensure_oauth_client_in_tenant(
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

pub async fn submit_marketplace_app(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperMarketplaceAppSummary>, AppError> {
    let tenant_id = service::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleMarketplaceSubmit,
    )
    .await?;

    ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;

    sqlx::query(
        r#"
        INSERT INTO developer_marketplace_apps (
          tenant_id,
          client_id,
          status,
          submitted_by,
          updated_at
        )
        VALUES ($1, $2, 'pending', $3, now())
        ON CONFLICT (tenant_id, client_id) DO UPDATE
        SET status = 'pending',
            submitted_by = EXCLUDED.submitted_by,
            review_reason = NULL,
            updated_at = now()
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    let app = get_marketplace_app_summary(&state.db, tenant_id, &client_id).await?;
    Ok(Json(app))
}

pub async fn review_marketplace_app(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
    Json(input): Json<ReviewMarketplaceAppInput>,
) -> Result<Json<DeveloperMarketplaceAppSummary>, AppError> {
    let tenant_id = service::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::MarketplaceReview,
    )
    .await?;

    ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;

    if !matches!(input.status.as_str(), "approved" | "rejected" | "suspended" | "pending") {
        return Err(AppError::bad_request(
            "invalid_marketplace_status",
            "Marketplace status must be 'approved', 'rejected', 'suspended', or 'pending'",
        ));
    }

    let result = sqlx::query(
        r#"
        UPDATE developer_marketplace_apps
        SET status = $3::text::developer_marketplace_status,
            reviewed_by = $4,
            review_reason = $5,
            updated_at = now()
        WHERE tenant_id = $1
          AND client_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(input.status)
    .bind(auth.user_id)
    .bind(input.review_reason)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::bad_request(
            "marketplace_app_not_submitted",
            "Marketplace app has not been submitted yet",
        ));
    }

    let app = get_marketplace_app_summary(&state.db, tenant_id, &client_id).await?;
    Ok(Json(app))
}

async fn get_marketplace_app_summary(
    db: &PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<DeveloperMarketplaceAppSummary, AppError> {
    sqlx::query_as(
        r#"
        SELECT
          m.client_id,
          c.name,
          m.status::text AS status,
          m.review_reason,
          m.created_at,
          m.updated_at
        FROM developer_marketplace_apps m
        INNER JOIN oauth_clients c
          ON c.tenant_id = m.tenant_id
         AND c.client_id = m.client_id
        WHERE m.tenant_id = $1
          AND m.client_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        AppError::not_found(
            "marketplace_app_not_found",
            "Marketplace app not found",
        )
    })
}
