use std::time::Duration;

use base64::Engine;
use tonic::{
    Code, Request,
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
};

use crate::pb::nvbes::identity::internal::v1::{
    IntrospectAccessTokenRequest, IntrospectAccessTokenResponse,
    identity_internal_service_client::IdentityInternalServiceClient,
};

const INTROSPECTION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct IdentityGrpcClient {
    client: IdentityInternalServiceClient<Channel>,
    authorization: MetadataValue<tonic::metadata::Ascii>,
}

impl IdentityGrpcClient {
    pub fn new(endpoint: String, client_id: String, client_secret: String) -> anyhow::Result<Self> {
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
        access_token: String,
        headers: &axum::http::HeaderMap,
    ) -> Result<IntrospectAccessTokenResponse, axum::http::StatusCode> {
        let client_ip = nvbes_core::http::client_ip::client_ip(headers).unwrap_or_default();
        let mut request = Request::new(IntrospectAccessTokenRequest {
            access_token,
            client_ip,
        });
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request.set_timeout(INTROSPECTION_TIMEOUT);
        self.client
            .clone()
            .introspect_access_token(request)
            .await
            .map_err(|error| match error.code() {
                Code::Unauthenticated => axum::http::StatusCode::UNAUTHORIZED,
                Code::PermissionDenied => axum::http::StatusCode::FORBIDDEN,
                _ => axum::http::StatusCode::BAD_GATEWAY,
            })
            .map(tonic::Response::into_inner)
    }
}
