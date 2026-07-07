use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domains::developer::types::{
        DeveloperConsentScreenResponse, ReviewMarketplaceAppInput,
        UpsertDeveloperConsentScreenInput,
    },
    grpc_pb::nvbes::developer::v1::{
        GetConsentScreenRequest, GetPublicConsentScreenRequest, ListMarketplaceAppsRequest,
        ReviewMarketplaceAppRequest, SubmitMarketplaceAppRequest, UpsertConsentScreenRequest,
    },
    http::error::AppError,
};

use super::{developer_client, grpc_error, parse_optional_time, parse_time, request_context};

pub struct DeveloperMarketplaceAppRecord {
    pub client_id: String,
    pub status: String,
    pub review_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn list_marketplace_apps(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<Vec<DeveloperMarketplaceAppRecord>, AppError> {
    let mut client = developer_client().await?;
    let response = client
        .list_marketplace_apps(ListMarketplaceAppsRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    response
        .apps
        .into_iter()
        .map(marketplace_app_record)
        .collect()
}

pub async fn submit_marketplace_app(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    client_id: String,
) -> Result<DeveloperMarketplaceAppRecord, AppError> {
    let mut client = developer_client().await?;
    let app = client
        .submit_marketplace_app(SubmitMarketplaceAppRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            client_id,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    marketplace_app_record(app)
}

pub async fn review_marketplace_app(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    client_id: String,
    input: ReviewMarketplaceAppInput,
) -> Result<DeveloperMarketplaceAppRecord, AppError> {
    let mut client = developer_client().await?;
    let app = client
        .review_marketplace_app(ReviewMarketplaceAppRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            client_id,
            status: input.status,
            review_reason: input.review_reason.unwrap_or_default(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    marketplace_app_record(app)
}

pub async fn get_consent_screen(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    client_id: String,
) -> Result<DeveloperConsentScreenResponse, AppError> {
    let mut client = developer_client().await?;
    let screen = client
        .get_consent_screen(GetConsentScreenRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            client_id,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    consent_screen_response(screen)
}

pub async fn get_public_consent_screen(
    tenant_id: Uuid,
    client_id: String,
) -> Result<DeveloperConsentScreenResponse, AppError> {
    let mut client = developer_client().await?;
    let screen = client
        .get_public_consent_screen(GetPublicConsentScreenRequest {
            context: Some(system_request_context(tenant_id)),
            tenant_id: tenant_id.to_string(),
            client_id,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    consent_screen_response(screen)
}

fn system_request_context(tenant_id: Uuid) -> crate::grpc_pb::nvbes::platform::v1::RequestContext {
    crate::grpc_pb::nvbes::platform::v1::RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: Uuid::nil().to_string(),
        tenant: Some(crate::grpc_pb::nvbes::platform::v1::TenantContext {
            tenant_id: tenant_id.to_string(),
            workspace_id: String::new(),
            region_id: String::new(),
            data_residency: String::new(),
        }),
    }
}

pub async fn upsert_consent_screen(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    client_id: String,
    input: UpsertDeveloperConsentScreenInput,
) -> Result<DeveloperConsentScreenResponse, AppError> {
    let mut client = developer_client().await?;
    let screen = client
        .upsert_consent_screen(UpsertConsentScreenRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
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
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    consent_screen_response(screen)
}

fn marketplace_app_record(
    app: crate::grpc_pb::nvbes::developer::v1::MarketplaceApp,
) -> Result<DeveloperMarketplaceAppRecord, AppError> {
    Ok(DeveloperMarketplaceAppRecord {
        client_id: app.client_id,
        status: app.status,
        review_reason: empty_to_option(app.review_reason),
        created_at: parse_time(&app.created_at, "created_at")?,
        updated_at: parse_time(&app.updated_at, "updated_at")?,
    })
}

fn consent_screen_response(
    screen: crate::grpc_pb::nvbes::developer::v1::ConsentScreen,
) -> Result<DeveloperConsentScreenResponse, AppError> {
    Ok(DeveloperConsentScreenResponse {
        client_id: screen.client_id,
        product_name: screen.product_name,
        logo_url: empty_to_option(screen.logo_url),
        support_url: empty_to_option(screen.support_url),
        privacy_url: empty_to_option(screen.privacy_url),
        terms_url: empty_to_option(screen.terms_url),
        description: screen.description,
        brand_color: empty_to_option(screen.brand_color),
        custom_css: empty_to_option(screen.custom_css),
        help_text: empty_to_option(screen.help_text),
        configured: screen.configured,
        updated_at: parse_optional_time(&screen.updated_at, "updated_at")?,
    })
}

fn empty_to_option(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}
