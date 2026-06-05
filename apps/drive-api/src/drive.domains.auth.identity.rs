use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::http::error::AppError;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static INTROSPECTION_CACHE: OnceLock<
    RwLock<HashMap<String, (IdentityIntrospectionResponse, Instant)>>,
> = OnceLock::new();

fn get_introspection_cache()
-> &'static RwLock<HashMap<String, (IdentityIntrospectionResponse, Instant)>> {
    INTROSPECTION_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn get_cached_introspection(token_hash: &str) -> Option<IdentityIntrospectionResponse> {
    let cache = get_introspection_cache().read().ok()?;
    if let Some((response, expires_at)) = cache.get(token_hash) {
        if Instant::now() < *expires_at {
            return Some(response.clone());
        }
    }
    None
}

pub fn set_cached_introspection(
    token_hash: String,
    response: IdentityIntrospectionResponse,
    ttl: Duration,
) {
    if let Ok(mut cache) = get_introspection_cache().write() {
        cache.insert(token_hash, (response, Instant::now() + ttl));
    }
}

pub fn invalidate_cached_session(session_id: &str) {
    if let Ok(mut cache) = get_introspection_cache().write() {
        cache.retain(|_, (resp, _)| resp.sid.as_deref() != Some(session_id));
    }
}

pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

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

#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IdentityIntrospectionResponse {
    pub active: bool,
    pub scope: Option<String>,
    pub client_id: Option<String>,
    pub principal_type: Option<String>,
    pub token_type: Option<String>,
    pub sub: Option<String>,
    pub role: Option<String>,
    pub tenant_id: Option<uuid::Uuid>,
    pub organization_id: Option<uuid::Uuid>,
    pub workspace_id: Option<uuid::Uuid>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    #[serde(alias = "display_name")]
    pub name: Option<String>,
    pub acr: Option<String>,
    #[serde(default)]
    pub amr: Vec<String>,
    pub auth_time: Option<i64>,
    pub jti: Option<String>,
    pub sid: Option<String>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub nbf: Option<i64>,
    #[serde(default)]
    pub act: Option<serde_json::Value>,
    pub actor_principal_type: Option<String>,
    pub actor_role: Option<String>,
    pub actor_workspace_id: Option<uuid::Uuid>,
    pub actor_organization_id: Option<uuid::Uuid>,
    pub actor_tenant_id: Option<uuid::Uuid>,
    pub network_valid: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct IdentityTokenExchangeResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
    pub issued_token_type: Option<String>,
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

    #[allow(dead_code)]
    pub async fn exchange_token(
        &self,
        subject_token: &str,
        subject_token_type: Option<&str>,
        scope: Option<&str>,
    ) -> Result<IdentityTokenExchangeResponse, AppError> {
        let response = self
            .http
            .post(format!("{}/oauth/token", self.base_url))
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .json(&serde_json::json!({
                "grant_type": "urn:ietf:params:oauth:grant-type:token-exchange",
                "subject_token": subject_token,
                "subject_token_type": subject_token_type.unwrap_or("urn:ietf:params:oauth:token-type:access_token"),
                "scope": scope,
            }))
            .send()
            .await
            .map_err(|err| {
                AppError::internal(
                    "identity_token_exchange_failed",
                    &format!("Failed to contact Identity: {err}"),
                )
            })?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(AppError::unauthorized(
                "invalid_grant",
                "Token exchange failed: invalid subject token or client credentials.",
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::internal(
                "identity_token_exchange_failed",
                &format!("Token exchange failed ({status}): {body}"),
            ));
        }

        response
            .json::<IdentityTokenExchangeResponse>()
            .await
            .map_err(|err| {
                AppError::internal(
                    "identity_token_exchange_invalid_response",
                    &format!("Token exchange returned invalid JSON: {err}"),
                )
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspection_cache_operations() {
        let token = "test_token_123";
        let token_hash = hash_token(token);

        let response = IdentityIntrospectionResponse {
            active: true,
            scope: Some("read write".to_string()),
            client_id: Some("client1".to_string()),
            principal_type: Some("user".to_string()),
            token_type: Some("access_token".to_string()),
            sub: Some("user1".to_string()),
            role: Some("admin".to_string()),
            tenant_id: None,
            organization_id: None,
            workspace_id: None,
            username: None,
            email: None,
            email_verified: None,
            name: None,
            acr: None,
            amr: vec![],
            auth_time: None,
            jti: None,
            sid: Some("session123".to_string()),
            exp: None,
            iat: None,
            nbf: None,
            act: None,
            actor_principal_type: None,
            actor_role: None,
            actor_workspace_id: None,
            actor_organization_id: None,
            actor_tenant_id: None,
            network_valid: None,
        };

        // Cache miss initially
        assert!(get_cached_introspection(&token_hash).is_none());

        // Cache hit after store
        set_cached_introspection(token_hash.clone(), response.clone(), Duration::from_secs(5));
        let cached = get_cached_introspection(&token_hash).expect("cache hit");
        assert_eq!(cached.sid, Some("session123".to_string()));

        // Invalidation by session ID
        invalidate_cached_session("session123");
        assert!(get_cached_introspection(&token_hash).is_none());

        // Cache expiration test
        set_cached_introspection(token_hash.clone(), response, Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(2));
        assert!(get_cached_introspection(&token_hash).is_none());
    }
}
