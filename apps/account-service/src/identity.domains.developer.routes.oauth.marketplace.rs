use std::collections::HashMap;

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        grpc::{self, DeveloperMarketplaceAppRecord},
        rbac::DeveloperPermission,
        service,
        types::{
            DeveloperMarketplaceAppSummary, DeveloperMarketplaceAppsResponse,
            ReviewMarketplaceAppInput,
        },
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

use super::consent::{ensure_oauth_client_in_tenant, oauth_client_name_in_tenant};

pub async fn list_marketplace_apps(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperMarketplaceAppsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let records = grpc::list_marketplace_apps(tenant_id, auth.user_id).await?;
    let names = oauth_client_names_in_tenant(
        &state.db,
        tenant_id,
        records
            .iter()
            .map(|record| record.client_id.as_str())
            .collect(),
    )
    .await?;
    let apps = records
        .into_iter()
        .filter_map(|record| {
            names
                .get(&record.client_id)
                .map(|name| marketplace_summary(record, name.clone()))
        })
        .collect();

    Ok(Json(DeveloperMarketplaceAppsResponse { apps }))
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

    let name = oauth_client_name_in_tenant(&state.db, tenant_id, &client_id).await?;
    let record = grpc::submit_marketplace_app(tenant_id, auth.user_id, client_id).await?;
    Ok(Json(marketplace_summary(record, name)))
}

pub async fn review_marketplace_app(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(client_id): Path<String>,
    Json(input): Json<ReviewMarketplaceAppInput>,
) -> Result<Json<DeveloperMarketplaceAppSummary>, AppError> {
    let tenant_id =
        service::require_permission(&state.db, &auth, DeveloperPermission::MarketplaceReview)
            .await?;

    let name = oauth_client_name_in_tenant(&state.db, tenant_id, &client_id).await?;
    ensure_oauth_client_in_tenant(&state.db, tenant_id, &client_id).await?;
    let record = grpc::review_marketplace_app(tenant_id, auth.user_id, client_id, input).await?;
    Ok(Json(marketplace_summary(record, name)))
}

async fn oauth_client_names_in_tenant(
    db: &PgPool,
    tenant_id: Uuid,
    client_ids: Vec<&str>,
) -> Result<HashMap<String, String>, AppError> {
    if client_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT client_id, name FROM oauth_clients WHERE tenant_id = $1 AND client_id = ANY($2)",
    )
    .bind(tenant_id)
    .bind(&client_ids)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().collect())
}

fn marketplace_summary(
    record: DeveloperMarketplaceAppRecord,
    name: String,
) -> DeveloperMarketplaceAppSummary {
    DeveloperMarketplaceAppSummary {
        client_id: record.client_id,
        name,
        status: record.status,
        review_reason: record.review_reason,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}
