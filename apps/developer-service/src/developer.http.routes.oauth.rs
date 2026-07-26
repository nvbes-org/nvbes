use std::collections::HashMap;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    access,
    app::DeveloperAppState,
    grpc::{
        client_metadata, consent, marketplace,
        pb::nvbes::developer::v1::{
            CreateScopeRequest, GetClientMetadataRequest, GetConsentScreenRequest,
            ReviewMarketplaceAppRequest, SubmitMarketplaceAppRequest, UpdateScopeRequest,
            UpsertConsentScreenRequest,
        },
        scopes,
    },
    http::{
        auth::DeveloperAuth,
        context::request_context,
        error::AppError,
        types::{
            CreateScopeInput, DeveloperConsentScreenResponse, DeveloperMarketplaceAppSummary,
            DeveloperMarketplaceAppsResponse, DeveloperOAuthClientSummary,
            DeveloperOAuthClientsResponse, DeveloperScopeRegistryEntry,
            DeveloperScopeRegistryResponse, ReviewMarketplaceAppInput, UpdateScopeInput,
            UpsertDeveloperConsentScreenInput,
        },
    },
    rbac::DeveloperPermission,
};

#[path = "developer.http.routes.oauth.views.rs"]
mod views;

use views::{consent_view, marketplace_view, non_empty, oauth_client_names, scope_view};

#[utoipa::path(get, path = "/developer/console/oauth-clients", tag = "developer-console", responses((status = 200, description = "OAuth clients")))]
pub async fn list_oauth_clients(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperOAuthClientsResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleAppsRead).await?;
    let clients = state
        .identity
        .list_oauth_clients(&auth.access_token)
        .await?
        .clients;
    let metadata = client_metadata::client_metadata(
        &state.db,
        GetClientMetadataRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            client_ids: clients
                .iter()
                .map(|client| client.client_id.clone())
                .collect(),
        },
    )
    .await?
    .metadata
    .into_iter()
    .map(|metadata| (metadata.client_id.clone(), metadata))
    .collect::<HashMap<_, _>>();

    let oauth_clients = clients
        .into_iter()
        .map(|client| {
            let metadata = metadata.get(&client.client_id);
            DeveloperOAuthClientSummary {
                client_id: client.client_id,
                name: client.name,
                status: "active".to_string(),
                marketplace_status: metadata
                    .and_then(|item| non_empty(item.marketplace_status.as_str())),
                consent_screen_configured: metadata
                    .is_some_and(|item| item.consent_screen_configured),
                redirect_uri_count: i64::try_from(client.redirect_uris.len()).unwrap_or(i64::MAX),
                allowed_scopes: Vec::new(),
                health_status: metadata
                    .map(|item| item.health_status.clone())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        })
        .collect();

    Ok(Json(DeveloperOAuthClientsResponse { oauth_clients }))
}

#[utoipa::path(get, path = "/developer/console/marketplace/apps", tag = "developer-console", responses((status = 200, description = "Marketplace apps")))]
pub async fn list_marketplace_apps(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperMarketplaceAppsResponse>, AppError> {
    access::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleMarketplaceRead,
    )
    .await?;
    let response = marketplace::list_marketplace_apps(&state.db, auth.tenant_id).await?;
    let names = oauth_client_names(&state.db, auth.tenant_id).await?;
    let apps = response
        .apps
        .into_iter()
        .map(|app| marketplace_view(app, &names))
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(DeveloperMarketplaceAppsResponse { apps }))
}

#[utoipa::path(post, path = "/developer/console/marketplace/apps/{clientId}/submit", tag = "developer-console", responses((status = 200, description = "Marketplace app submitted")))]
pub async fn submit_marketplace_app(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperMarketplaceAppSummary>, AppError> {
    access::require_permission(
        &state.db,
        &auth,
        DeveloperPermission::ConsoleMarketplaceSubmit,
    )
    .await?;
    let app = marketplace::submit_marketplace_app(
        &state.db,
        SubmitMarketplaceAppRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            client_id,
        },
    )
    .await?;
    let names = oauth_client_names(&state.db, auth.tenant_id).await?;
    Ok(Json(marketplace_view(app, &names)?))
}

#[utoipa::path(post, path = "/developer/console/marketplace/apps/{clientId}/review", tag = "developer-console", responses((status = 200, description = "Marketplace app reviewed")))]
pub async fn review_marketplace_app(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
    Json(input): Json<ReviewMarketplaceAppInput>,
) -> Result<Json<DeveloperMarketplaceAppSummary>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::MarketplaceReview).await?;
    let app = marketplace::review_marketplace_app(
        &state.db,
        ReviewMarketplaceAppRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            client_id,
            status: input.status,
            review_reason: input.review_reason.unwrap_or_default(),
        },
    )
    .await?;
    let names = oauth_client_names(&state.db, auth.tenant_id).await?;
    Ok(Json(marketplace_view(app, &names)?))
}

#[utoipa::path(get, path = "/developer/console/scopes", tag = "developer-console", responses((status = 200, description = "Developer scopes")))]
pub async fn list_scopes(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
) -> Result<Json<DeveloperScopeRegistryResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesRead).await?;
    let response = scopes::list_scopes(&state.db).await?;
    Ok(Json(DeveloperScopeRegistryResponse {
        scopes: response.scopes.into_iter().map(scope_view).collect(),
    }))
}

#[utoipa::path(post, path = "/developer/console/scopes", tag = "developer-console", responses((status = 200, description = "Developer scope created")))]
pub async fn create_scope(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Json(input): Json<CreateScopeInput>,
) -> Result<Json<DeveloperScopeRegistryEntry>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage).await?;
    let scope = scopes::create_scope(
        &state.db,
        CreateScopeRequest {
            context: Some(request_context(&auth)),
            scope_key: input.scope_key,
            display_name: input.display_name,
            description: input.description,
            risk: input.risk,
            owner_team: input.owner_team,
            lifecycle: input.lifecycle.unwrap_or_else(|| "proposed".to_string()),
            allowed_audiences: input.allowed_audiences,
        },
    )
    .await?;
    Ok(Json(scope_view(scope)))
}

#[utoipa::path(put, path = "/developer/console/scopes/{scopeKey}", tag = "developer-console", responses((status = 200, description = "Developer scope updated")))]
pub async fn update_scope(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(scope_key): Path<String>,
    Json(input): Json<UpdateScopeInput>,
) -> Result<Json<DeveloperScopeRegistryEntry>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage).await?;
    let scope = scopes::update_scope(
        &state.db,
        UpdateScopeRequest {
            context: Some(request_context(&auth)),
            scope_key,
            display_name: input.display_name,
            description: input.description,
            risk: input.risk,
            owner_team: input.owner_team,
            lifecycle: input.lifecycle,
            allowed_audiences: input.allowed_audiences,
        },
    )
    .await?;
    Ok(Json(scope_view(scope)))
}

#[utoipa::path(delete, path = "/developer/console/scopes/{scopeKey}", tag = "developer-console", responses((status = 204, description = "Developer scope deleted")))]
pub async fn delete_scope(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(scope_key): Path<String>,
) -> Result<StatusCode, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleScopesManage).await?;
    scopes::delete_scope(&state.db, scope_key).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/developer/console/oauth-clients/{clientId}/consent-screen", tag = "developer-console", responses((status = 200, description = "Consent screen")))]
pub async fn get_consent_screen(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
) -> Result<Json<DeveloperConsentScreenResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleAppsRead).await?;
    let screen = consent::get_consent_screen(
        &state.db,
        GetConsentScreenRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            client_id,
        },
    )
    .await?;
    Ok(Json(consent_view(screen)?))
}

#[utoipa::path(put, path = "/developer/console/oauth-clients/{clientId}/consent-screen", tag = "developer-console", responses((status = 200, description = "Consent screen updated")))]
pub async fn upsert_consent_screen(
    State(state): State<DeveloperAppState>,
    Extension(auth): Extension<DeveloperAuth>,
    Path(client_id): Path<String>,
    Json(input): Json<UpsertDeveloperConsentScreenInput>,
) -> Result<Json<DeveloperConsentScreenResponse>, AppError> {
    access::require_permission(&state.db, &auth, DeveloperPermission::ConsoleAppsUpdate).await?;
    let screen = consent::upsert_consent_screen(
        &state.db,
        UpsertConsentScreenRequest {
            context: Some(request_context(&auth)),
            tenant_id: auth.tenant_id.to_string(),
            client_id,
            product_name: input.product_name,
            logo_url: input.logo_url.unwrap_or_default(),
            support_url: input.support_url.unwrap_or_default(),
            privacy_url: input.privacy_url.unwrap_or_default(),
            terms_url: input.terms_url.unwrap_or_default(),
            description: input.description,
            brand_color: input.brand_color.unwrap_or_default(),
            custom_css: input.custom_css.unwrap_or_default(),
            help_text: input.help_text.unwrap_or_default(),
        },
    )
    .await?;
    Ok(Json(consent_view(screen)?))
}
