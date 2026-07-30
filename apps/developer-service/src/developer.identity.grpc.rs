use std::time::Duration;

use base64::Engine;
use tonic::{
    Code, Request,
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
};
use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::identity::internal::v1::{
        IntrospectAccessTokenRequest, IntrospectAccessTokenResponse,
        identity_internal_service_client::IdentityInternalServiceClient,
    },
    http::error::AppError,
    identity::IdentityClaims,
};

const IDENTITY_GRPC_ENDPOINT_ENV: &str = "NVBES_IDENTITY_GRPC_ENDPOINT";
const IDENTITY_CLIENT_ID_ENV: &str = "NVBES_DEVELOPER_IDENTITY_CLIENT_ID";
const IDENTITY_CLIENT_SECRET_ENV: &str = "NVBES_DEVELOPER_IDENTITY_CLIENT_SECRET";
const INTROSPECTION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct IdentityGrpcClient {
    client: IdentityInternalServiceClient<Channel>,
    authorization: MetadataValue<tonic::metadata::Ascii>,
}

impl IdentityGrpcClient {
    pub fn from_env() -> anyhow::Result<Self> {
        let endpoint = std::env::var(IDENTITY_GRPC_ENDPOINT_ENV)
            .unwrap_or_else(|_| "http://127.0.0.1:4010".to_string());
        let client_id = required_env(IDENTITY_CLIENT_ID_ENV)?;
        let client_secret = required_env(IDENTITY_CLIENT_SECRET_ENV)?;
        let channel = Endpoint::from_shared(endpoint)?.connect_lazy();
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(format!("{client_id}:{client_secret}"));
        let authorization = format!("Basic {encoded}")
            .parse()
            .map_err(|_| anyhow::anyhow!("Identity gRPC credential contains invalid metadata"))?;
        Ok(Self {
            client: IdentityInternalServiceClient::new(channel),
            authorization,
        })
    }

    pub async fn introspect(
        &self,
        token: &str,
        incoming_headers: Option<&axum::http::HeaderMap>,
    ) -> Result<IdentityClaims, AppError> {
        let client_ip = incoming_headers
            .and_then(nvbes_core::http::client_ip::client_ip)
            .unwrap_or_default();
        let mut request = Request::new(IntrospectAccessTokenRequest {
            access_token: token.to_string(),
            client_ip,
        });
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request.set_timeout(INTROSPECTION_TIMEOUT);
        let response = self
            .client
            .clone()
            .introspect_access_token(request)
            .await
            .map_err(grpc_error)?
            .into_inner();
        convert_response(response)
    }
}

fn convert_response(value: IntrospectAccessTokenResponse) -> Result<IdentityClaims, AppError> {
    Ok(IdentityClaims {
        active: value.active,
        scope: value.scope,
        client_id: value.client_id,
        principal_type: value.principal_type,
        token_type: value.token_type,
        audience: value.audience,
        sub: value.sub,
        tenant_id: optional_uuid(value.tenant_id, "tenant_id")?,
        organization_id: optional_uuid(value.organization_id, "organization_id")?,
        workspace_id: optional_uuid(value.workspace_id, "workspace_id")?,
        email: value.email,
        display_name: value.display_name,
        acr: value.acr,
        amr: value.amr,
        auth_time: value.auth_time,
        sid: value.sid,
        exp: value.exp,
        iat: value.iat,
        nbf: value.nbf,
        network_valid: value.network_valid,
    })
}

fn optional_uuid(value: Option<String>, field: &str) -> Result<Option<Uuid>, AppError> {
    value
        .map(|value| {
            Uuid::parse_str(&value).map_err(|_| {
                AppError::internal(
                    "identity_grpc_response_invalid",
                    format!("Identity gRPC response contains an invalid {field}."),
                )
            })
        })
        .transpose()
}

fn required_env(name: &'static str) -> anyhow::Result<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

fn grpc_error(error: tonic::Status) -> AppError {
    match error.code() {
        Code::Unauthenticated => AppError::unauthorized("invalid_token", error.message()),
        Code::PermissionDenied => {
            AppError::forbidden("identity_introspection_forbidden", error.message())
        }
        _ => AppError::internal("identity_introspection_failed", error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{IntrospectAccessTokenResponse, convert_response};

    #[test]
    fn maps_identity_display_name() {
        let response = IntrospectAccessTokenResponse {
            display_name: Some("Ada Lovelace".to_string()),
            ..Default::default()
        };

        assert_eq!(
            convert_response(response).unwrap().display_name.as_deref(),
            Some("Ada Lovelace")
        );
    }
}
