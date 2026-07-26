use chrono::{DateTime, Utc};
use tonic::{Code, transport::Channel};
use uuid::Uuid;

use crate::{
    grpc_pb::nvbes::{
        developer::v1::{
            CountStaleSecretsRequest, DeveloperCredentialSummary as GrpcCredential,
            GetPublicConsentScreenRequest, ListCredentialSummariesRequest,
            RevokeSecretVersionRequest, VerifyClientSecretVersionRequest,
            developer_service_client::DeveloperServiceClient,
        },
        platform::v1::{RequestContext, TenantContext},
    },
    http::error::AppError,
};

const DEVELOPER_GRPC_ENDPOINT_ENV: &str = "NVBES_DEVELOPER_GRPC_ENDPOINT";

#[derive(Debug)]
pub struct DeveloperConsentScreen {
    pub product_name: String,
    pub logo_url: Option<String>,
    pub support_url: Option<String>,
    pub privacy_url: Option<String>,
    pub terms_url: Option<String>,
    pub description: String,
    pub brand_color: Option<String>,
    pub custom_css: Option<String>,
    pub help_text: Option<String>,
}

#[derive(Debug)]
pub struct DeveloperCredentialSummary {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub status: String,
    pub secret_last4: String,
    pub owner_email: Option<String>,
    pub scopes: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub async fn get_public_consent_screen(
    tenant_id: Uuid,
    client_id: String,
) -> Result<DeveloperConsentScreen, AppError> {
    let screen = client()
        .await?
        .get_public_consent_screen(GetPublicConsentScreenRequest {
            context: Some(request_context(tenant_id, Uuid::nil())),
            tenant_id: tenant_id.to_string(),
            client_id,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(DeveloperConsentScreen {
        product_name: screen.product_name,
        logo_url: optional_text(screen.logo_url),
        support_url: optional_text(screen.support_url),
        privacy_url: optional_text(screen.privacy_url),
        terms_url: optional_text(screen.terms_url),
        description: screen.description,
        brand_color: optional_text(screen.brand_color),
        custom_css: optional_text(screen.custom_css),
        help_text: optional_text(screen.help_text),
    })
}

pub async fn verify_client_secret_version(
    tenant_id: Uuid,
    client_id: &str,
    client_secret: &str,
) -> Result<bool, AppError> {
    Ok(client()
        .await?
        .verify_client_secret_version(VerifyClientSecretVersionRequest {
            context: Some(request_context(tenant_id, Uuid::nil())),
            tenant_id: tenant_id.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner()
        .valid)
}

pub async fn list_credential_summaries(
    tenant_id: Uuid,
    actor_id: Uuid,
) -> Result<Vec<DeveloperCredentialSummary>, AppError> {
    client()
        .await?
        .list_credential_summaries(ListCredentialSummariesRequest {
            context: Some(request_context(tenant_id, actor_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner()
        .credentials
        .into_iter()
        .map(credential)
        .collect()
}

pub async fn count_stale_secrets(tenant_id: Uuid, actor_id: Uuid) -> Result<i64, AppError> {
    Ok(client()
        .await?
        .count_stale_secrets(CountStaleSecretsRequest {
            context: Some(request_context(tenant_id, actor_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner()
        .stale_secret_count)
}

pub async fn revoke_secret_version(
    tenant_id: Uuid,
    actor_id: Uuid,
    version_id: Uuid,
) -> Result<String, AppError> {
    Ok(client()
        .await?
        .revoke_secret_version(RevokeSecretVersionRequest {
            context: Some(request_context(tenant_id, actor_id)),
            tenant_id: tenant_id.to_string(),
            version_id: version_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner()
        .client_id)
}

async fn client() -> Result<DeveloperServiceClient<Channel>, AppError> {
    let endpoint = std::env::var(DEVELOPER_GRPC_ENDPOINT_ENV)
        .unwrap_or_else(|_| "http://127.0.0.1:4041".to_string());
    if endpoint.trim().is_empty() {
        return Err(AppError::internal(
            "developer_grpc_endpoint_invalid",
            format!("{DEVELOPER_GRPC_ENDPOINT_ENV} must not be empty."),
        ));
    }
    DeveloperServiceClient::connect(endpoint)
        .await
        .map_err(|error| AppError::internal("developer_grpc_connect_failed", error.to_string()))
}

fn request_context(tenant_id: Uuid, actor_id: Uuid) -> RequestContext {
    let request_id = Uuid::new_v4().to_string();
    RequestContext {
        request_id: request_id.clone(),
        correlation_id: request_id,
        actor_principal_id: actor_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: tenant_id.to_string(),
            workspace_id: String::new(),
            region_id: String::new(),
            data_residency: String::new(),
        }),
    }
}

fn credential(value: GrpcCredential) -> Result<DeveloperCredentialSummary, AppError> {
    Ok(DeveloperCredentialSummary {
        id: parse_uuid(&value.id, "credential id")?,
        client_id: value.client_id,
        name: value.name,
        status: value.status,
        secret_last4: value.secret_last4,
        owner_email: optional_text(value.owner_email),
        scopes: value.scopes,
        last_used_at: optional_time(&value.last_used_at, "last_used_at")?,
        created_at: time(&value.created_at, "created_at")?,
        expires_at: optional_time(&value.expires_at, "expires_at")?,
    })
}

fn grpc_error(status: tonic::Status) -> AppError {
    match status.code() {
        Code::InvalidArgument => {
            AppError::bad_request("developer_request_invalid", status.message())
        }
        Code::NotFound => AppError::not_found("developer_resource_not_found", status.message()),
        Code::PermissionDenied => AppError::forbidden("developer_access_denied", status.message()),
        Code::Unauthenticated => AppError::unauthorized("developer_unauthorized", status.message()),
        Code::AlreadyExists | Code::Aborted => {
            AppError::conflict("developer_conflict", status.message())
        }
        _ => AppError::internal("developer_grpc_failed", status.message()),
    }
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|error| {
        AppError::internal(
            "developer_contract_invalid",
            format!("{field} is not a UUID: {error}"),
        )
    })
}

fn time(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "developer_contract_invalid",
                format!("{field} is not RFC3339: {error}"),
            )
        })
}

fn optional_time(value: &str, field: &'static str) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.is_empty() {
        Ok(None)
    } else {
        time(value, field).map(Some)
    }
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}
