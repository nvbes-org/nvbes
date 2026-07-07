use std::{future::Future, net::SocketAddr};

use tonic::{Request, Response, Status, transport::Server};

use crate::{
    app::DeveloperAppState,
    grpc::{
        activity_logs, client_metadata, consent, credentials, health, marketplace, overview,
        pb::nvbes::{
            developer::v1 as developer,
            developer::v1::developer_service_server::{DeveloperService, DeveloperServiceServer},
        },
        rbac, sandbox, scopes, secrets,
        service_status::{parse_uuid, validate_context},
        tokens, webhooks,
    },
};

#[derive(Clone)]
pub struct DeveloperGrpcService {
    state: DeveloperAppState,
}

impl DeveloperGrpcService {
    pub fn new(state: DeveloperAppState) -> Self {
        Self { state }
    }

    pub fn into_server(self) -> DeveloperServiceServer<Self> {
        DeveloperServiceServer::new(self)
    }

    fn unsupported<T>(&self, operation: &'static str) -> Result<Response<T>, Status> {
        let _ = &self.state;
        Err(Status::unimplemented(format!(
            "Developer {operation} RPC is not implemented by developer-service yet"
        )))
    }
}

pub async fn serve(
    addr: SocketAddr,
    state: DeveloperAppState,
    shutdown: impl Future<Output = ()>,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(DeveloperGrpcService::new(state).into_server())
        .serve_with_shutdown(addr, shutdown)
        .await
}

macro_rules! unsupported_rpc {
    ($service:expr, $request:expr, $operation:literal) => {{
        validate_context($request.into_inner().context.as_ref())?;
        $service.unsupported($operation)
    }};
}

macro_rules! delegate_request {
    ($service:expr, $request:expr, $handler:path) => {{
        let request = $request.into_inner();
        validate_context(request.context.as_ref())?;
        Ok(Response::new($handler(&$service.state.db, request).await?))
    }};
}

#[tonic::async_trait]
impl DeveloperService for DeveloperGrpcService {
    async fn list_apps(
        &self,
        request: Request<developer::ListAppsRequest>,
    ) -> Result<Response<developer::ListAppsResponse>, Status> {
        unsupported_rpc!(self, request, "list apps")
    }

    async fn create_app(
        &self,
        request: Request<developer::CreateAppRequest>,
    ) -> Result<Response<developer::DeveloperApp>, Status> {
        unsupported_rpc!(self, request, "create app")
    }

    async fn update_app(
        &self,
        request: Request<developer::UpdateAppRequest>,
    ) -> Result<Response<developer::DeveloperApp>, Status> {
        unsupported_rpc!(self, request, "update app")
    }

    async fn rotate_app_secret(
        &self,
        request: Request<developer::RotateAppSecretRequest>,
    ) -> Result<Response<developer::AppSecret>, Status> {
        unsupported_rpc!(self, request, "rotate app secret")
    }

    async fn delete_app(
        &self,
        request: Request<developer::DeleteAppRequest>,
    ) -> Result<Response<developer::AppDeletion>, Status> {
        unsupported_rpc!(self, request, "delete app")
    }

    async fn get_overview_summary(
        &self,
        request: Request<developer::GetOverviewSummaryRequest>,
    ) -> Result<Response<developer::DeveloperOverviewSummary>, Status> {
        delegate_request!(self, request, overview::overview_summary)
    }

    async fn get_client_metadata(
        &self,
        request: Request<developer::GetClientMetadataRequest>,
    ) -> Result<Response<developer::GetClientMetadataResponse>, Status> {
        delegate_request!(self, request, client_metadata::client_metadata)
    }

    async fn list_developer_roles(
        &self,
        request: Request<developer::ListDeveloperRolesRequest>,
    ) -> Result<Response<developer::ListDeveloperRolesResponse>, Status> {
        delegate_request!(self, request, rbac::list_developer_roles)
    }

    async fn list_webhook_endpoints(
        &self,
        request: Request<developer::ListWebhookEndpointsRequest>,
    ) -> Result<Response<developer::ListWebhookEndpointsResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        Ok(Response::new(
            webhooks::list_webhook_endpoints(&self.state.db, tenant_id).await?,
        ))
    }

    async fn create_webhook_endpoint(
        &self,
        request: Request<developer::CreateWebhookEndpointRequest>,
    ) -> Result<Response<developer::WebhookEndpoint>, Status> {
        let request = request.into_inner();
        let context = request
            .context
            .clone()
            .ok_or_else(|| Status::invalid_argument("request context is required"))?;
        validate_context(Some(&context))?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
        Ok(Response::new(
            webhooks::create_webhook_endpoint(&self.state.db, tenant_id, actor_id, request).await?,
        ))
    }

    async fn update_webhook_endpoint(
        &self,
        request: Request<developer::UpdateWebhookEndpointRequest>,
    ) -> Result<Response<developer::WebhookEndpoint>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        Ok(Response::new(
            webhooks::update_webhook_endpoint(&self.state.db, tenant_id, request).await?,
        ))
    }

    async fn delete_webhook_endpoint(
        &self,
        request: Request<developer::DeleteWebhookEndpointRequest>,
    ) -> Result<Response<developer::WebhookEndpointDeletion>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        let endpoint_id = parse_uuid(&request.endpoint_id, "endpoint_id")?;
        Ok(Response::new(
            webhooks::delete_webhook_endpoint(&self.state.db, tenant_id, endpoint_id).await?,
        ))
    }

    async fn replay_webhook_delivery(
        &self,
        request: Request<developer::ReplayWebhookDeliveryRequest>,
    ) -> Result<Response<developer::WebhookDelivery>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        let delivery_id = parse_uuid(&request.delivery_id, "delivery_id")?;
        Ok(Response::new(
            webhooks::replay_webhook_delivery(&self.state.db, tenant_id, delivery_id).await?,
        ))
    }

    async fn list_scopes(
        &self,
        request: Request<developer::ListScopesRequest>,
    ) -> Result<Response<developer::ListScopesResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        Ok(Response::new(scopes::list_scopes(&self.state.db).await?))
    }

    async fn create_scope(
        &self,
        request: Request<developer::CreateScopeRequest>,
    ) -> Result<Response<developer::DeveloperScope>, Status> {
        delegate_request!(self, request, scopes::create_scope)
    }

    async fn update_scope(
        &self,
        request: Request<developer::UpdateScopeRequest>,
    ) -> Result<Response<developer::DeveloperScope>, Status> {
        delegate_request!(self, request, scopes::update_scope)
    }

    async fn delete_scope(
        &self,
        request: Request<developer::DeleteScopeRequest>,
    ) -> Result<Response<developer::ScopeDeletion>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        Ok(Response::new(
            scopes::delete_scope(&self.state.db, request.scope_key).await?,
        ))
    }

    async fn list_webhook_deliveries(
        &self,
        request: Request<developer::ListWebhookDeliveriesRequest>,
    ) -> Result<Response<developer::ListWebhookDeliveriesResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        let endpoint_id = parse_uuid(&request.endpoint_id, "endpoint_id")?;
        Ok(Response::new(
            webhooks::list_webhook_deliveries(&self.state.db, tenant_id, endpoint_id).await?,
        ))
    }

    async fn create_developer_token(
        &self,
        request: Request<developer::CreateDeveloperTokenRequest>,
    ) -> Result<Response<developer::DeveloperToken>, Status> {
        unsupported_rpc!(self, request, "create developer token")
    }

    async fn revoke_developer_token(
        &self,
        request: Request<developer::RevokeDeveloperTokenRequest>,
    ) -> Result<Response<developer::TokenRevocation>, Status> {
        unsupported_rpc!(self, request, "revoke developer token")
    }

    async fn record_token_debug_session(
        &self,
        request: Request<developer::RecordTokenDebugSessionRequest>,
    ) -> Result<Response<developer::TokenDebugSession>, Status> {
        let request = request.into_inner();
        let context = request
            .context
            .clone()
            .ok_or_else(|| Status::invalid_argument("request context is required"))?;
        validate_context(Some(&context))?;
        let actor_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
        Ok(Response::new(
            tokens::record_token_debug_session(&self.state.db, actor_id, request).await?,
        ))
    }

    async fn create_sandbox(
        &self,
        request: Request<developer::CreateSandboxRequest>,
    ) -> Result<Response<developer::Sandbox>, Status> {
        delegate_request!(self, request, sandbox::create_sandbox)
    }

    async fn reset_sandbox(
        &self,
        request: Request<developer::ResetSandboxRequest>,
    ) -> Result<Response<developer::Sandbox>, Status> {
        delegate_request!(self, request, sandbox::reset_sandbox)
    }

    async fn get_sandbox(
        &self,
        request: Request<developer::GetSandboxRequest>,
    ) -> Result<Response<developer::GetSandboxResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        Ok(Response::new(
            sandbox::get_sandbox(&self.state.db, tenant_id).await?,
        ))
    }

    async fn list_api_logs(
        &self,
        request: Request<developer::ListApiLogsRequest>,
    ) -> Result<Response<developer::ListApiLogsResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        Ok(Response::new(
            webhooks::list_api_logs(&self.state.db, tenant_id).await?,
        ))
    }

    async fn list_activity_logs(
        &self,
        request: Request<developer::ListActivityLogsRequest>,
    ) -> Result<Response<developer::ListActivityLogsResponse>, Status> {
        delegate_request!(self, request, activity_logs::list_activity_logs)
    }

    async fn list_health_checks(
        &self,
        request: Request<developer::ListHealthChecksRequest>,
    ) -> Result<Response<developer::ListHealthChecksResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        Ok(Response::new(
            health::list_health_checks(&self.state.db, tenant_id).await?,
        ))
    }

    async fn run_health_checks(
        &self,
        request: Request<developer::RunHealthChecksRequest>,
    ) -> Result<Response<developer::RunHealthChecksResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        Ok(Response::new(
            health::run_health_checks(&self.state.db, tenant_id).await?,
        ))
    }

    async fn list_marketplace_apps(
        &self,
        request: Request<developer::ListMarketplaceAppsRequest>,
    ) -> Result<Response<developer::ListMarketplaceAppsResponse>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
        Ok(Response::new(
            marketplace::list_marketplace_apps(&self.state.db, tenant_id).await?,
        ))
    }

    async fn submit_marketplace_app(
        &self,
        request: Request<developer::SubmitMarketplaceAppRequest>,
    ) -> Result<Response<developer::MarketplaceApp>, Status> {
        delegate_request!(self, request, marketplace::submit_marketplace_app)
    }

    async fn review_marketplace_app(
        &self,
        request: Request<developer::ReviewMarketplaceAppRequest>,
    ) -> Result<Response<developer::MarketplaceApp>, Status> {
        delegate_request!(self, request, marketplace::review_marketplace_app)
    }

    async fn get_consent_screen(
        &self,
        request: Request<developer::GetConsentScreenRequest>,
    ) -> Result<Response<developer::ConsentScreen>, Status> {
        delegate_request!(self, request, consent::get_consent_screen)
    }

    async fn get_public_consent_screen(
        &self,
        request: Request<developer::GetPublicConsentScreenRequest>,
    ) -> Result<Response<developer::ConsentScreen>, Status> {
        delegate_request!(self, request, consent::get_public_consent_screen)
    }

    async fn upsert_consent_screen(
        &self,
        request: Request<developer::UpsertConsentScreenRequest>,
    ) -> Result<Response<developer::ConsentScreen>, Status> {
        delegate_request!(self, request, consent::upsert_consent_screen)
    }

    async fn list_secret_versions(
        &self,
        request: Request<developer::ListSecretVersionsRequest>,
    ) -> Result<Response<developer::ListSecretVersionsResponse>, Status> {
        delegate_request!(self, request, secrets::list_secret_versions)
    }

    async fn list_credential_summaries(
        &self,
        request: Request<developer::ListCredentialSummariesRequest>,
    ) -> Result<Response<developer::ListCredentialSummariesResponse>, Status> {
        delegate_request!(self, request, credentials::list_credential_summaries)
    }

    async fn count_stale_secrets(
        &self,
        request: Request<developer::CountStaleSecretsRequest>,
    ) -> Result<Response<developer::CountStaleSecretsResponse>, Status> {
        delegate_request!(self, request, credentials::count_stale_secrets)
    }

    async fn verify_client_secret_version(
        &self,
        request: Request<developer::VerifyClientSecretVersionRequest>,
    ) -> Result<Response<developer::VerifyClientSecretVersionResponse>, Status> {
        delegate_request!(self, request, credentials::verify_client_secret_version)
    }

    async fn record_secret_rotation(
        &self,
        request: Request<developer::RecordSecretRotationRequest>,
    ) -> Result<Response<developer::SecretRotation>, Status> {
        let request = request.into_inner();
        validate_context(request.context.as_ref())?;
        let actor_id = parse_uuid(
            &request
                .context
                .as_ref()
                .ok_or_else(|| Status::invalid_argument("request context is required"))?
                .actor_principal_id,
            "actor_principal_id",
        )?;
        Ok(Response::new(
            secrets::record_secret_rotation(&self.state.db, actor_id, request).await?,
        ))
    }

    async fn revoke_secret_version(
        &self,
        request: Request<developer::RevokeSecretVersionRequest>,
    ) -> Result<Response<developer::SecretVersionRevocation>, Status> {
        delegate_request!(self, request, secrets::revoke_secret_version)
    }
}
