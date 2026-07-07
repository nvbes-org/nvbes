use axum::{Extension, Json, extract::State};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app::AppState,
    domains::developer::{
        grpc, service,
        types::{DeveloperOAuthClientSummary, DeveloperOAuthClientsResponse},
    },
    http::{error::AppError, middleware::jwt::AuthContext},
};

pub async fn list_oauth_clients(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeveloperOAuthClientsResponse>, AppError> {
    let tenant_id = service::require_tenant_id(&auth)?;
    let oauth_clients = list_oauth_client_summaries(&state.db, tenant_id, auth.user_id).await?;
    Ok(Json(DeveloperOAuthClientsResponse { oauth_clients }))
}

async fn list_oauth_client_summaries(
    db: &PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<Vec<DeveloperOAuthClientSummary>, AppError> {
    let mut oauth_clients = sqlx::query_as::<_, DeveloperOAuthClientSummary>(
        r#"
        SELECT
          c.client_id,
          c.name,
          CASE WHEN c.revoked_at IS NULL THEN 'active' ELSE 'revoked' END AS status,
          NULL::text AS marketplace_status,
          false AS consent_screen_configured,
          cardinality(c.redirect_uris)::bigint AS redirect_uri_count,
          COALESCE(policy.allowed_scopes, '{}'::text[]) AS allowed_scopes,
          'unknown'::text AS health_status
        FROM oauth_clients c
        LEFT JOIN LATERAL (
          SELECT array_agg(DISTINCT scope ORDER BY scope) AS allowed_scopes
          FROM oauth_client_policies p
          CROSS JOIN LATERAL unnest(p.allowed_scopes) AS scope
          WHERE p.client_id = c.id
            AND p.status = 'active'
        ) policy ON true
        WHERE c.tenant_id = $1
        ORDER BY c.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    let metadata = grpc::get_client_metadata(
        tenant_id,
        actor_principal_id,
        oauth_clients
            .iter()
            .map(|client| client.client_id.clone())
            .collect(),
    )
    .await?;

    for client in &mut oauth_clients {
        if let Some(metadata) = metadata.get(&client.client_id) {
            client.marketplace_status = metadata.marketplace_status.clone();
            client.consent_screen_configured = metadata.consent_screen_configured;
            client.health_status = metadata.health_status.clone();
        }
    }

    Ok(oauth_clients)
}
