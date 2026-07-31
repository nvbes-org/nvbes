use crate::{app::AppState, http::error::AppError};
use axum::{Extension, Form, Json, Router, extract::State, http::HeaderMap, routing::post};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Deserialize;
use sqlx::Row;
use std::time::Duration;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/introspect", post(introspect))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct IntrospectRequest {
    pub(crate) token: String,
    pub(crate) token_type_hint: Option<String>,
    pub(crate) client_id: Option<String>,
    pub(crate) client_secret: Option<String>,
    pub(crate) client_assertion_type: Option<String>,
    pub(crate) client_assertion: Option<String>,
}

#[utoipa::path(
    post,
    path = "/oauth/introspect",
    tag = "oauth",
    request_body = IntrospectRequest,
    responses(
        (status = 200, description = "Introspection result", body = crate::domains::oauth::service::IntrospectionResponse),
        (status = 401, description = "Unauthorized", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn introspect(
    State(state): State<AppState>,
    headers: HeaderMap,
    dpop: Option<Extension<crate::http::middleware::dpop::DpopContext>>,
    mtls: Option<Extension<crate::http::mtls::MtlsCertificateThumbprint>>,
    Form(request): Form<IntrospectRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut client_auth = super::token::token_client_auth(
        &headers,
        request.client_id.as_deref(),
        request.client_secret.as_deref(),
        request.client_assertion_type.as_deref(),
        request.client_assertion.as_deref(),
    )?;
    let endpoint = format!(
        "{}/oauth/introspect",
        state.config.api_base_url.trim_end_matches('/')
    );
    client_auth.client_assertion_verified =
        crate::domains::oauth::client_assertion::verify_private_key_jwt(
            &state.db,
            &client_auth,
            &endpoint,
            state.config.api_base_url.trim_end_matches('/'),
        )
        .await?;
    let security =
        crate::domains::oauth::profiles::load_client_security(&state.db, &client_auth.client_id)
            .await?;
    super::token::enforce_token_endpoint_security(
        &state,
        &client_auth,
        &security,
        dpop.as_ref().map(|Extension(context)| context),
        mtls.as_ref().map(|Extension(thumbprint)| thumbprint),
    )?;

    nvbes_core::limiter::check_dual_rate_limit(
        &state.redis,
        &headers,
        "oauth_introspect",
        &client_auth.client_id,
        60,
        30,
        Duration::from_secs(60),
    )
    .await?;

    let client_row = sqlx::query(
        r#"
        SELECT client_secret_hash, tenant_id, revoked_at
        FROM oauth_clients
        WHERE client_id = $1
        LIMIT 1
        "#,
    )
    .bind(&client_auth.client_id)
    .fetch_optional(&state.db)
    .await?;

    let Some(client_row) = client_row else {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The OAuth client is not registered.",
        ));
    };

    if client_row
        .get::<Option<chrono::DateTime<chrono::Utc>>, _>("revoked_at")
        .is_some()
    {
        return Err(AppError::unauthorized(
            "invalid_client",
            "The OAuth client is not registered.",
        ));
    }

    let client_secret_hash: String = client_row.get("client_secret_hash");
    let client_tenant_id: Uuid = client_row.get("tenant_id");
    if !client_auth.client_assertion_verified {
        let client_secret = client_auth.client_secret.as_deref().ok_or_else(|| {
            AppError::unauthorized("invalid_client", "Client authentication is required.")
        })?;
        crate::domains::oauth::verify_client_secret_with_overlap(
            client_tenant_id,
            &client_auth.client_id,
            client_secret,
            &client_secret_hash,
        )
        .await?;
    }

    let client_ip = nvbes_core::http::client_ip::client_ip(&headers);
    let response = crate::domains::oauth::flows::introspect_token(
        &state.db,
        &state.redis,
        &state.jwt,
        &client_auth.client_id,
        &request.token,
        request.token_type_hint,
        client_ip,
    )
    .await?;

    Ok(Json(serde_json::to_value(response)?))
}
