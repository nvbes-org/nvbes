use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Request},
    middleware::Next,
    routing::{get, post},
};

use nvbes_core::http::error::ErrorEnvelope;

use crate::{
    app::AppState,
    http::{
        error::AppError,
        request::{client_ip, user_agent},
    },
};

use nvbes_core::auth::token_hash;

use super::{PublicDownloadUrlResponse, PublicShareResponse};

pub fn router(_state: &AppState) -> Router<AppState> {
    let cached_get = Router::new()
        .route("/public/shares/{token}", get(get_public_share))
        .layer(axum::middleware::from_fn(
            |req: Request<Body>, next: Next| async {
                let mut res = next.run(req).await;
                nvbes_core::security::insert_cdn_cache_headers(res.headers_mut(), 60, 300);
                res
            },
        ));

    cached_get.route(
        "/public/shares/{token}/download-url",
        post(create_public_download_url),
    )
}

#[utoipa::path(
    get,
    path = "/public/shares/{token}",
    tag = "public-shares",
    params(
        ("token" = String, Path, description = "Share link token"),
    ),
    responses(
        (status = 200, description = "Public share details", body = PublicShareResponse),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn get_public_share(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<PublicShareResponse>, AppError> {
    let client_key = public_client_key(&headers, &token);
    let result = crate::domains::share_links::get_public_share(
        &state.db,
        &state.redis,
        &token,
        &client_key,
        crate::domains::share_links::logic::public_share_requires_clean_scan(
            &state.config.environment,
        ),
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/public/shares/{token}/download-url",
    tag = "public-shares",
    params(
        ("token" = String, Path, description = "Share link token"),
    ),
    responses(
        (status = 200, description = "Public download URL created", body = PublicDownloadUrlResponse),
        (status = 404, description = "Not found", body = ErrorEnvelope),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
async fn create_public_download_url(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<PublicDownloadUrlResponse>, AppError> {
    let client_key = public_client_key(&headers, &token);
    let result = crate::domains::share_links::create_public_download_url(
        &state.db,
        &state.redis,
        state.storage.as_ref(),
        &token,
        &client_key,
        crate::domains::share_links::logic::public_share_requires_clean_scan(
            &state.config.environment,
        ),
        client_ip(&headers),
        user_agent(&headers),
    )
    .await?;
    Ok(Json(result))
}

fn public_client_key(headers: &HeaderMap, token: &str) -> String {
    let token_key = token_hash(token.trim());
    match client_ip(headers) {
        Some(ip) => format!("ip:{ip}:token:{token_key}"),
        None => format!("token:{token_key}"),
    }
}

#[cfg(test)]
mod tests {
    use super::public_client_key;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn public_client_key_hashes_token_when_ip_is_missing() {
        let headers = HeaderMap::new();
        let token = "share-token-123";

        let client_key = public_client_key(&headers, token);

        assert!(client_key.starts_with("token:"));
        assert!(!client_key.contains(token));
    }

    #[test]
    fn public_client_key_prefers_ip_over_token_hash() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::HeaderName::from_static(
                nvbes_core::http::client_ip::TRUSTED_CLIENT_IP_HEADER,
            ),
            HeaderValue::from_static("203.0.113.7"),
        );

        let client_key = public_client_key(&headers, "share-token-123");

        assert!(client_key.starts_with("ip:203.0.113.7:token:"));
        assert!(!client_key.contains("share-token-123"));
    }
}
