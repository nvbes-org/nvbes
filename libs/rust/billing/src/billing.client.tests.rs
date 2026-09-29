use std::sync::Mutex;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Response, Status, transport::Server};

use super::{BillingClient, BillingClientConfig, BillingClientError, map_status};
use crate::proto::nvbes::billing::v1::{
    BillingEventReceipt, CreateBillingPortalSessionRequest, CreateBillingPortalSessionResponse,
    CreateCheckoutSessionRequest, CreateCheckoutSessionResponse,
    GetWorkspaceBillingOverviewRequest, GetWorkspaceEntitlementsRequest, SubmitBillingEventRequest,
    WorkspaceBillingOverviewResponse, WorkspaceEntitlementsResponse,
    billing_delivery_service_server::{BillingDeliveryService, BillingDeliveryServiceServer},
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

const TOKEN: &str = "01234567890123456789012345678901";

#[test]
fn billing_client_config_from_values_validates_auth_token_length() {
    let err = BillingClientConfig::from_values(
        "development",
        "http://127.0.0.1:50051".to_string(),
        "short".to_string(),
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(err, BillingClientError::Configuration(_)));
}

#[test]
fn billing_client_config_from_values_rejects_zero_timeout() {
    let err = BillingClientConfig::from_values(
        "development",
        "http://127.0.0.1:50051".to_string(),
        TOKEN.to_string(),
        Duration::ZERO,
    )
    .unwrap_err();
    assert!(matches!(err, BillingClientError::Configuration(_)));
}

#[test]
fn billing_client_config_requires_https_outside_development() {
    let err = BillingClientConfig::from_values(
        "production",
        "http://127.0.0.1:50051".to_string(),
        TOKEN.to_string(),
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(err, BillingClientError::Configuration(_)));
}

#[test]
fn billing_client_config_accepts_https_endpoint() {
    BillingClientConfig::from_values(
        "production",
        "https://billing.internal:50051".to_string(),
        TOKEN.to_string(),
        Duration::from_secs(5),
    )
    .expect("https endpoint should validate");
}

#[test]
fn billing_client_config_accepts_http_in_test_environment() {
    BillingClientConfig::from_values(
        "test",
        "http://127.0.0.1:50051".to_string(),
        TOKEN.to_string(),
        Duration::from_secs(5),
    )
    .expect("test environment allows http");
}

#[test]
fn billing_client_config_rejects_invalid_endpoint_uri() {
    let err = BillingClientConfig::from_values(
        "development",
        "not a uri".to_string(),
        TOKEN.to_string(),
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(err, BillingClientError::Configuration(_)));
}

#[test]
fn billing_client_config_rejects_metadata_unsafe_token() {
    let mut unsafe_token = TOKEN.to_string();
    unsafe_token.push('\n');
    let err = BillingClientConfig::from_values(
        "development",
        "http://127.0.0.1:50051".to_string(),
        unsafe_token,
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(err, BillingClientError::Configuration(_)));
}

#[test]
fn billing_client_config_from_env_requires_endpoint_and_token() {
    let _lock = ENV_LOCK.lock().unwrap();
    let previous_endpoint = std::env::var_os("NVBES_BILLING_GRPC_ENDPOINT");
    let previous_token = std::env::var_os("NVBES_BILLING_GRPC_AUTH_TOKEN");
    unsafe {
        std::env::remove_var("NVBES_BILLING_GRPC_ENDPOINT");
        std::env::remove_var("NVBES_BILLING_GRPC_AUTH_TOKEN");
    }
    let missing_endpoint = BillingClientConfig::from_env("development").unwrap_err();
    assert!(matches!(
        missing_endpoint,
        BillingClientError::Configuration(_)
    ));

    unsafe {
        std::env::set_var("NVBES_BILLING_GRPC_ENDPOINT", "http://127.0.0.1:50051");
    }
    let missing_token = BillingClientConfig::from_env("development").unwrap_err();
    assert!(matches!(
        missing_token,
        BillingClientError::Configuration(_)
    ));

    unsafe {
        std::env::set_var("NVBES_BILLING_GRPC_AUTH_TOKEN", TOKEN);
    }
    BillingClientConfig::from_env("development").expect("complete env");

    restore_env("NVBES_BILLING_GRPC_ENDPOINT", previous_endpoint);
    restore_env("NVBES_BILLING_GRPC_AUTH_TOKEN", previous_token);
}

fn restore_env(name: &str, previous: Option<std::ffi::OsString>) {
    match previous {
        Some(value) => unsafe { std::env::set_var(name, value) },
        None => unsafe { std::env::remove_var(name) },
    }
}

#[test]
fn map_status_classifies_grpc_codes() {
    assert!(matches!(
        map_status(Status::new(Code::Unauthenticated, "")),
        BillingClientError::Unauthorized
    ));
    assert!(matches!(
        map_status(Status::new(Code::PermissionDenied, "")),
        BillingClientError::Unauthorized
    ));
    assert!(matches!(
        map_status(Status::new(Code::Unavailable, "")),
        BillingClientError::Unavailable
    ));
    assert!(matches!(
        map_status(Status::new(Code::DeadlineExceeded, "")),
        BillingClientError::Unavailable
    ));
    assert!(matches!(
        map_status(Status::new(Code::ResourceExhausted, "")),
        BillingClientError::Unavailable
    ));
    assert!(matches!(
        map_status(Status::new(Code::InvalidArgument, "")),
        BillingClientError::Protocol
    ));
}

#[tokio::test]
async fn billing_client_connect_marks_unreachable_endpoints_unavailable() {
    let config = BillingClientConfig::from_values(
        "development",
        "http://127.0.0.1:1".to_string(),
        TOKEN.to_string(),
        Duration::from_millis(200),
    )
    .expect("config");
    let err = BillingClient::connect(config).await.unwrap_err();
    assert!(matches!(err, BillingClientError::Unavailable));
}

#[tokio::test]
async fn billing_client_get_workspace_entitlements_over_loopback() {
    let (endpoint, server) = spawn_delivery_server(DeliveryOutcome::Entitlements).await;
    let config = BillingClientConfig::from_values(
        "development",
        endpoint,
        TOKEN.to_string(),
        Duration::from_secs(2),
    )
    .expect("config");
    let client = BillingClient::connect(config).await.expect("connect");
    let entitlements = client
        .get_workspace_entitlements("workspace-1".into())
        .await
        .expect("entitlements");
    assert_eq!(entitlements.workspace_id, "workspace-1");
    assert_eq!(entitlements.plan_code, "standard_monthly");
    server.abort();
}

#[tokio::test]
async fn billing_client_maps_unauthorized_entitlement_responses() {
    let (endpoint, server) =
        spawn_delivery_server(DeliveryOutcome::Status(Code::Unauthenticated)).await;
    let config = BillingClientConfig::from_values(
        "development",
        endpoint,
        TOKEN.to_string(),
        Duration::from_secs(2),
    )
    .expect("config");
    let client = BillingClient::connect(config).await.expect("connect");
    let err = client
        .get_workspace_entitlements("workspace-1".into())
        .await
        .unwrap_err();
    assert!(matches!(err, BillingClientError::Unauthorized));
    server.abort();
}

#[derive(Clone)]
enum DeliveryOutcome {
    Entitlements,
    Status(Code),
}

#[derive(Clone)]
struct DeliveryService {
    outcome: DeliveryOutcome,
}

#[tonic::async_trait]
impl BillingDeliveryService for DeliveryService {
    async fn get_workspace_entitlements(
        &self,
        request: Request<GetWorkspaceEntitlementsRequest>,
    ) -> Result<Response<WorkspaceEntitlementsResponse>, Status> {
        match &self.outcome {
            DeliveryOutcome::Entitlements => Ok(Response::new(WorkspaceEntitlementsResponse {
                workspace_id: request.into_inner().workspace_id,
                plan_code: "standard_monthly".into(),
                status: "active".into(),
                max_seats: 5,
                storage_bytes_quota: 0,
                api_rate_limit_per_minute: 0,
                custom_domain_enabled: false,
                advanced_security_enabled: false,
                current_period_end: None,
            })),
            DeliveryOutcome::Status(code) => Err(Status::new(*code, "controlled")),
        }
    }

    async fn get_workspace_billing_overview(
        &self,
        _request: Request<GetWorkspaceBillingOverviewRequest>,
    ) -> Result<Response<WorkspaceBillingOverviewResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn create_checkout_session(
        &self,
        _request: Request<CreateCheckoutSessionRequest>,
    ) -> Result<Response<CreateCheckoutSessionResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn create_billing_portal_session(
        &self,
        _request: Request<CreateBillingPortalSessionRequest>,
    ) -> Result<Response<CreateBillingPortalSessionResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }

    async fn submit_billing_event(
        &self,
        _request: Request<SubmitBillingEventRequest>,
    ) -> Result<Response<BillingEventReceipt>, Status> {
        Err(Status::unimplemented("unused"))
    }
}

async fn spawn_delivery_server(outcome: DeliveryOutcome) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind gRPC");
    let address = listener.local_addr().expect("addr");
    let service = DeliveryService { outcome };
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(BillingDeliveryServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("serve");
    });
    (format!("http://{address}"), handle)
}
