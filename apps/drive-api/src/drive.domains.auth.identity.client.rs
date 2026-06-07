use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::http::StatusCode;

use crate::http::error::AppError;

use super::{
    cache::{get_cached_introspection, hash_token, set_cached_introspection},
    types::IdentityIntrospectionResponse,
};

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> reqwest::Client {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(5))
                .build()
                .expect("identity auth client should initialize")
        })
        .clone()
}

fn build_mtls_http_client() -> Result<reqwest::Client, String> {
    let cert_path = std::env::var("NVBES_MTLS_CLIENT_CERT_PATH")
        .map_err(|_| "NVBES_MTLS_CLIENT_CERT_PATH is required".to_string())?;
    let key_path = std::env::var("NVBES_MTLS_CLIENT_KEY_PATH")
        .map_err(|_| "NVBES_MTLS_CLIENT_KEY_PATH is required".to_string())?;
    let identity = nvbes_core::tls::build_mtls_identity(&cert_path, &key_path)?;
    reqwest::Client::builder()
        .identity(identity)
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build mTLS HTTP client: {e}"))
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}

#[derive(Clone)]
pub struct IdentityAuthClient {
    http: reqwest::Client,
    base_url: String,
    client_id: String,
    client_secret: String,
}

impl IdentityAuthClient {
    pub fn from_env() -> Result<Self, AppError> {
        let mtls_enabled = env_bool("NVBES_MTLS_ENABLED", false);
        let base_url = if mtls_enabled {
            std::env::var("NVBES_IDENTITY_MTLS_BASE_URL").unwrap_or_else(|_| {
                let port = std::env::var("NVBES_MTLS_PORT")
                    .ok()
                    .and_then(|v| v.parse::<u16>().ok())
                    .unwrap_or(4001);
                format!("https://localhost:{port}")
            })
        } else {
            std::env::var("NVBES_IDENTITY_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string())
        };
        let client_id = std::env::var("NVBES_IDENTITY_CLIENT_ID").map_err(|_| {
            AppError::internal(
                "identity_client_id_missing",
                "NVBES_IDENTITY_CLIENT_ID is required to validate Identity tokens.",
            )
        })?;
        let client_secret = std::env::var("NVBES_IDENTITY_CLIENT_SECRET").map_err(|_| {
            AppError::internal(
                "identity_client_secret_missing",
                "NVBES_IDENTITY_CLIENT_SECRET is required to validate Identity tokens.",
            )
        })?;

        let http = if mtls_enabled {
            build_mtls_http_client()
                .map_err(|e| AppError::internal("mtls_client_init_failed", &e))?
        } else {
            http_client()
        };

        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            client_id,
            client_secret,
        })
    }

    pub async fn introspect_access_token(
        &self,
        token: &str,
        request_headers: Option<&axum::http::HeaderMap>,
    ) -> Result<IdentityIntrospectionResponse, AppError> {
        let token_hash = hash_token(token);
        if let Some(cached) = get_cached_introspection(&token_hash) {
            metrics::counter!("drive_identity_introspection_cached_total").increment(1);
            return Ok(cached);
        }

        let started_at = Instant::now();
        metrics::counter!("drive_identity_introspection_total").increment(1);
        let mut builder = self
            .http
            .post(format!("{}/oauth/introspect", self.base_url));

        if let Some(headers) = request_headers {
            let mut outgoing = axum::http::HeaderMap::new();
            nvbes_observability::propagate_headers_trace_context(headers, &mut outgoing);
            if let Some(ip) = headers.get("x-nvbes-client-ip") {
                outgoing.insert("x-nvbes-client-ip", ip.clone());
            }
            builder = builder.headers(outgoing);
        } else {
            builder = nvbes_core::trace_context::with_fresh_trace_headers(builder);
        }

        let response = builder
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .json(&serde_json::json!({
                "token": token,
                "token_type_hint": "access_token",
            }))
            .send()
            .await
            .map_err(|err| {
                AppError::internal(
                    "identity_introspection_failed",
                    &format!("Failed to contact Identity: {err}"),
                )
            })?;

        if response.status() == StatusCode::UNAUTHORIZED {
            metrics::histogram!("drive_identity_introspection_duration_seconds")
                .record(started_at.elapsed().as_secs_f64());
            return Err(AppError::unauthorized(
                "invalid_token",
                "The access token is invalid or expired.",
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            metrics::histogram!("drive_identity_introspection_duration_seconds")
                .record(started_at.elapsed().as_secs_f64());
            return Err(AppError::internal(
                "identity_introspection_failed",
                &format!("Identity introspection failed ({status}): {body}"),
            ));
        }

        let parsed = response
            .json::<IdentityIntrospectionResponse>()
            .await
            .map_err(|err| {
                AppError::internal(
                    "identity_introspection_invalid_response",
                    &format!("Identity introspection returned invalid JSON: {err}"),
                )
            })?;
        metrics::histogram!("drive_identity_introspection_duration_seconds")
            .record(started_at.elapsed().as_secs_f64());

        if parsed.active {
            set_cached_introspection(token_hash, parsed.clone(), Duration::from_secs(30));
        }

        Ok(parsed)
    }
    pub async fn create_workspace(
        &self,
        token: &str,
        name: &str,
        request_headers: Option<&axum::http::HeaderMap>,
    ) -> Result<serde_json::Value, AppError> {
        let mut builder = self
            .http
            .post(format!("{}/workspaces", self.base_url))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "name": name,
            }));

        if let Some(headers) = request_headers {
            let mut outgoing = axum::http::HeaderMap::new();
            nvbes_observability::propagate_headers_trace_context(headers, &mut outgoing);
            builder = builder.headers(outgoing);
        } else {
            builder = nvbes_core::trace_context::with_fresh_trace_headers(builder);
        }

        let response = builder.send().await.map_err(|err| {
            AppError::internal(
                "identity_create_workspace_failed",
                &format!("Failed to contact Identity: {err}"),
            )
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::internal(
                "identity_create_workspace_failed",
                &format!("Identity workspace creation failed ({status}): {body}"),
            ));
        }

        let body = response.json::<serde_json::Value>().await.map_err(|err| {
            AppError::internal(
                "identity_create_workspace_invalid_response",
                &format!("Identity workspace creation returned invalid JSON: {err}"),
            )
        })?;

        Ok(body)
    }
}
