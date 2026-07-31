use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use sqlx::Row;
use tonic::{Request, Response, Status};

use crate::grpc::pb::nvbes::identity::internal::v1::{
    IntrospectAccessTokenRequest, IntrospectAccessTokenResponse,
    identity_internal_service_server::{IdentityInternalService, IdentityInternalServiceServer},
};

use super::{MockAccountState, account_client_row, decode_test_claims};

pub(super) fn spawn(state: MockAccountState) {
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("test gRPC server should bind");
    let addr = listener.local_addr().expect("gRPC listener addr");
    drop(listener);
    unsafe {
        std::env::set_var("NVBES_IDENTITY_GRPC_ENDPOINT", format!("http://{addr}"));
    }
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(IdentityInternalServiceServer::new(
                MockIdentityGrpcService { state },
            ))
            .serve(addr)
            .await
            .expect("test gRPC server should run");
    });
}

#[derive(Clone)]
struct MockIdentityGrpcService {
    state: MockAccountState,
}

#[tonic::async_trait]
impl IdentityInternalService for MockIdentityGrpcService {
    async fn introspect_access_token(
        &self,
        request: Request<IntrospectAccessTokenRequest>,
    ) -> Result<Response<IntrospectAccessTokenResponse>, Status> {
        let (client_id, client_secret) = basic_auth(&request)?;
        verify_client(&self.state, &client_id, &client_secret).await?;
        let claims = decode_test_claims(
            &self.state.key,
            &self.state.issuer,
            &request.into_inner().access_token,
        )
        .map_err(|_| Status::unauthenticated("invalid access token"))?;
        let token_row = account_client_row(
            &self.state.pool,
            claims.client_id.as_deref().unwrap_or_default(),
        )
        .await
        .map_err(|_| Status::unauthenticated("invalid access token"))?;
        let client_revoked = token_row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
            .ok()
            .flatten()
            .is_some();
        let principal_status: String = token_row.get("principal_status");

        Ok(Response::new(IntrospectAccessTokenResponse {
            active: !client_revoked && principal_status == "active",
            scope: Some(claims.scope),
            client_id: claims.client_id,
            principal_type: Some("service_account".to_string()),
            token_type: Some("access_token".to_string()),
            audience: Some(claims.aud),
            sub: Some(claims.sub),
            role: Some(claims.role),
            tenant_id: claims.tenant_id,
            organization_id: claims.organization_id,
            workspace_id: claims.workspace_id,
            email_verified: Some(true),
            display_name: Some("Drive E2E Robot".to_string()),
            acr: Some("aal1".to_string()),
            amr: claims.amr,
            auth_time: Some(claims.iat),
            jti: Some(claims.jti),
            sid: Some(claims.sid),
            exp: Some(claims.exp),
            iat: Some(claims.iat),
            nbf: Some(claims.nbf),
            network_valid: Some(true),
            ..Default::default()
        }))
    }
}

async fn verify_client(
    state: &MockAccountState,
    client_id: &str,
    client_secret: &str,
) -> Result<(), Status> {
    let row = account_client_row(&state.pool, client_id)
        .await
        .map_err(|_| Status::unauthenticated("invalid client"))?;
    let secret_hash: String = row.get("client_secret_hash");
    nvbes_product_identity::oauth::verify_client_secret(client_secret, &secret_hash)
        .map_err(|_| Status::unauthenticated("invalid client"))
}

fn basic_auth<T>(request: &Request<T>) -> Result<(String, String), Status> {
    let value = request
        .metadata()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Basic "))
        .ok_or_else(|| Status::unauthenticated("missing client credential"))?;
    let decoded = STANDARD
        .decode(value)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| Status::unauthenticated("invalid client credential"))?;
    let (client_id, client_secret) = decoded
        .split_once(':')
        .ok_or_else(|| Status::unauthenticated("invalid client credential"))?;
    Ok((client_id.to_string(), client_secret.to_string()))
}
