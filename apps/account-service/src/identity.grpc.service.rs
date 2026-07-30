use std::{future::Future, net::SocketAddr};

use tonic::{Request, Response, Status, transport::Server};

use crate::{
    app::AppState,
    grpc::{auth, conversions},
    grpc_pb::nvbes::identity::internal::v1::{
        IntrospectAccessTokenRequest, IntrospectAccessTokenResponse,
        identity_internal_service_server::{
            IdentityInternalService, IdentityInternalServiceServer,
        },
    },
};

const MAX_ACCESS_TOKEN_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct IdentityGrpcService {
    state: AppState,
}

impl IdentityGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

pub async fn serve(
    addr: SocketAddr,
    state: AppState,
    shutdown: impl Future<Output = ()>,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(IdentityInternalServiceServer::new(
            IdentityGrpcService::new(state),
        ))
        .serve_with_shutdown(addr, shutdown)
        .await
}

#[tonic::async_trait]
impl IdentityInternalService for IdentityGrpcService {
    async fn introspect_access_token(
        &self,
        request: Request<IntrospectAccessTokenRequest>,
    ) -> Result<Response<IntrospectAccessTokenResponse>, Status> {
        let client_id = auth::authenticate_client(&self.state.db, &request).await?;
        let request = request.into_inner();
        let access_token = request.access_token.trim();
        if access_token.is_empty() || access_token.len() > MAX_ACCESS_TOKEN_BYTES {
            return Err(Status::invalid_argument(
                "A valid access token is required.",
            ));
        }
        let client_ip = non_empty(request.client_ip);
        let introspection = crate::domains::oauth::flows::introspect_token(
            &self.state.db,
            &self.state.redis,
            &self.state.jwt,
            &client_id,
            access_token,
            Some("access_token".to_string()),
            client_ip,
        )
        .await
        .map_err(app_status)?;
        let response = conversions::introspection_response(introspection)
            .map_err(|_| Status::internal("Identity introspection response encoding failed."))?;
        Ok(Response::new(response))
    }
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn app_status(error: crate::http::error::AppError) -> Status {
    match error.status {
        axum::http::StatusCode::BAD_REQUEST => Status::invalid_argument(error.message),
        axum::http::StatusCode::UNAUTHORIZED => Status::unauthenticated(error.message),
        axum::http::StatusCode::FORBIDDEN => Status::permission_denied(error.message),
        _ => {
            tracing::error!(code = error.code, "identity gRPC introspection failed");
            Status::internal("Identity introspection failed.")
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn trims_optional_client_ip() {
        assert_eq!(
            super::non_empty(" 203.0.113.10 ".to_string()),
            Some("203.0.113.10".to_string())
        );
        assert_eq!(super::non_empty(" ".to_string()), None);
    }
}
