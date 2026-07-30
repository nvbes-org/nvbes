use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::http::error::AppError;

const IDENTITY_BASE_URL_ENV: &str = "NVBES_IDENTITY_SERVICE_BASE_URL";

#[derive(Clone)]
pub struct IdentityClient {
    http: Client,
    base_url: String,
    grpc: crate::identity_grpc::IdentityGrpcClient,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityClaims {
    pub active: bool,
    pub scope: Option<String>,
    pub client_id: Option<String>,
    pub principal_type: Option<String>,
    pub token_type: Option<String>,
    pub audience: Option<String>,
    pub sub: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub acr: Option<String>,
    #[serde(default)]
    pub amr: Vec<String>,
    pub auth_time: Option<i64>,
    pub sid: Option<String>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub nbf: Option<i64>,
    pub network_valid: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityOAuthClient {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub client_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct IdentityOAuthClients {
    pub clients: Vec<IdentityOAuthClient>,
}

#[derive(Debug, Deserialize)]
pub struct CreatedIdentityOAuthClient {
    pub client: IdentityOAuthClient,
    pub client_secret: String,
}

#[derive(Debug, Serialize)]
pub struct CreateIdentityOAuthClient<'a> {
    pub name: &'a str,
    pub redirect_uris: &'a [String],
    pub allowed_scopes: &'a [String],
    pub allowed_audiences: &'a [String],
    pub allowed_resources: &'a [String],
    pub required_acr: &'static str,
    pub client_type: &'static str,
    pub owner_scope_type: &'static str,
    pub owner_scope_id: Uuid,
    pub client_assertion_required: bool,
    pub requires_admin_consent: bool,
}

impl IdentityClient {
    pub fn from_env() -> anyhow::Result<Self> {
        let base_url = std::env::var(IDENTITY_BASE_URL_ENV)
            .unwrap_or_else(|_| "http://localhost:4000".to_string());
        Ok(Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            grpc: crate::identity_grpc::IdentityGrpcClient::from_env()?,
        })
    }

    pub async fn introspect(
        &self,
        token: &str,
        incoming_headers: Option<&axum::http::HeaderMap>,
    ) -> Result<IdentityClaims, AppError> {
        self.grpc.introspect(token, incoming_headers).await
    }

    pub async fn list_oauth_clients(
        &self,
        access_token: &str,
    ) -> Result<IdentityOAuthClients, AppError> {
        let response = self
            .http
            .get(format!("{}/oauth/clients", self.base_url))
            .bearer_auth(access_token)
            .send()
            .await?;
        self.decode(response, "identity_oauth_clients_failed").await
    }

    pub async fn create_oauth_client(
        &self,
        access_token: &str,
        request: &CreateIdentityOAuthClient<'_>,
    ) -> Result<CreatedIdentityOAuthClient, AppError> {
        let response = self
            .http
            .post(format!("{}/oauth/clients", self.base_url))
            .bearer_auth(access_token)
            .json(request)
            .send()
            .await?;
        self.decode(response, "identity_oauth_client_create_failed")
            .await
    }

    pub async fn revoke_oauth_client(
        &self,
        access_token: &str,
        client_id: &str,
    ) -> Result<(), AppError> {
        let response = self
            .http
            .delete(format!(
                "{}/oauth/clients/{}",
                self.base_url,
                urlencoding::encode(client_id)
            ))
            .bearer_auth(access_token)
            .send()
            .await?;
        self.ensure_success(response, "identity_oauth_client_revoke_failed")
            .await
    }

    pub async fn exchange_authorization_code(
        &self,
        client_id: &str,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<serde_json::Value, AppError> {
        let response = self
            .http
            .post(format!("{}/oauth/token", self.base_url))
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", client_id),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await?;
        self.decode(response, "identity_token_exchange_failed")
            .await
    }

    async fn decode<T: DeserializeOwned>(
        &self,
        response: Response,
        code: &'static str,
    ) -> Result<T, AppError> {
        if response.status().is_success() {
            return response
                .json::<T>()
                .await
                .map_err(|error| AppError::internal(code, error.to_string()));
        }
        Err(identity_response_error(response, code).await)
    }

    async fn ensure_success(&self, response: Response, code: &'static str) -> Result<(), AppError> {
        if response.status().is_success() {
            return Ok(());
        }
        Err(identity_response_error(response, code).await)
    }
}

async fn identity_response_error(response: Response, code: &'static str) -> AppError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| format!("Account service returned {status}."));

    match status {
        StatusCode::BAD_REQUEST => AppError::bad_request(code, message),
        StatusCode::UNAUTHORIZED => AppError::unauthorized(code, message),
        StatusCode::FORBIDDEN => AppError::forbidden(code, message),
        StatusCode::NOT_FOUND => AppError::not_found(code, message),
        StatusCode::CONFLICT => AppError::conflict(code, message),
        _ => AppError::internal(code, message),
    }
}
