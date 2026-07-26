use std::collections::HashMap;

use sqlx::Row;

use crate::{
    grpc::pb::nvbes::developer::v1::{ConsentScreen, DeveloperScope, MarketplaceApp},
    http::{
        context::{optional_string, optional_time, time},
        error::AppError,
        types::{
            DeveloperConsentScreenResponse, DeveloperMarketplaceAppSummary,
            DeveloperScopeRegistryEntry,
        },
    },
};

pub(super) async fn oauth_client_names(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
) -> Result<HashMap<String, String>, AppError> {
    Ok(
        sqlx::query("SELECT client_id, name FROM oauth_clients WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|row| (row.get("client_id"), row.get("name")))
            .collect(),
    )
}

pub(super) fn marketplace_view(
    app: MarketplaceApp,
    names: &HashMap<String, String>,
) -> Result<DeveloperMarketplaceAppSummary, AppError> {
    Ok(DeveloperMarketplaceAppSummary {
        name: names
            .get(&app.client_id)
            .cloned()
            .unwrap_or_else(|| app.client_id.clone()),
        client_id: app.client_id,
        status: app.status,
        review_reason: optional_string(app.review_reason),
        created_at: time(&app.created_at, "created_at")?,
        updated_at: time(&app.updated_at, "updated_at")?,
    })
}

pub(super) fn scope_view(scope: DeveloperScope) -> DeveloperScopeRegistryEntry {
    DeveloperScopeRegistryEntry {
        scope_key: scope.scope_key,
        display_name: scope.display_name,
        description: scope.description,
        risk: scope.risk,
        owner_team: scope.owner_team,
        lifecycle: scope.lifecycle,
        allowed_audiences: scope.allowed_audiences,
    }
}

pub(super) fn consent_view(
    screen: ConsentScreen,
) -> Result<DeveloperConsentScreenResponse, AppError> {
    Ok(DeveloperConsentScreenResponse {
        client_id: screen.client_id,
        product_name: screen.product_name,
        logo_url: optional_string(screen.logo_url),
        support_url: optional_string(screen.support_url),
        privacy_url: optional_string(screen.privacy_url),
        terms_url: optional_string(screen.terms_url),
        description: screen.description,
        brand_color: optional_string(screen.brand_color),
        custom_css: optional_string(screen.custom_css),
        help_text: optional_string(screen.help_text),
        configured: screen.configured,
        updated_at: optional_time(&screen.updated_at, "updated_at")?,
    })
}

pub(super) fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}
