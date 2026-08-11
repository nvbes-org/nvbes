use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{net::TcpListener, task::JoinHandle};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Response, Status, transport::Server};

use crate::proto::nvbes::email::v1::{
    ApplyEmailSuppressionRequest, EmailOperationsSnapshot, EmailOperatorActionReceipt,
    EmailPrivacyActivity, GetEmailOperationsSnapshotRequest, GetEmailPrivacyActivityRequest,
    ReleaseEmailSuppressionRequest, ReplayEmailRequest, ReviewEmailSuppressionRequest,
    email_operations_service_client::EmailOperationsServiceClient,
    email_operations_service_server::{EmailOperationsService, EmailOperationsServiceServer},
};
use crate::{EmailClientConfig, environment_test_support::ScopedEnvironment};

use super::{EmailOperationsClient, EmailOperationsError, map_status};

const TOKEN: &str = "operations-internal-token-more-than-32-characters";

#[derive(Clone, Default)]
struct OperationsService {
    calls: Arc<Mutex<Vec<(String, String)>>>,
}

impl OperationsService {
    fn record<T>(&self, operation: &str, request: &Request<T>) {
        let authorization = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        self.calls
            .lock()
            .unwrap()
            .push((operation.into(), authorization));
    }
}

#[tonic::async_trait]
impl EmailOperationsService for OperationsService {
    async fn get_operations_snapshot(
        &self,
        request: Request<GetEmailOperationsSnapshotRequest>,
    ) -> Result<Response<EmailOperationsSnapshot>, Status> {
        self.record("snapshot", &request);
        Ok(Response::new(EmailOperationsSnapshot {
            queued_message_count: 7,
            ..Default::default()
        }))
    }

    async fn replay_email(
        &self,
        request: Request<ReplayEmailRequest>,
    ) -> Result<Response<EmailOperatorActionReceipt>, Status> {
        self.record("replay", &request);
        Ok(Response::new(receipt("replay")))
    }

    async fn apply_suppression(
        &self,
        request: Request<ApplyEmailSuppressionRequest>,
    ) -> Result<Response<EmailOperatorActionReceipt>, Status> {
        self.record("apply", &request);
        Ok(Response::new(receipt("apply")))
    }

    async fn release_suppression(
        &self,
        request: Request<ReleaseEmailSuppressionRequest>,
    ) -> Result<Response<EmailOperatorActionReceipt>, Status> {
        self.record("release", &request);
        Ok(Response::new(receipt("release")))
    }

    async fn review_suppression(
        &self,
        request: Request<ReviewEmailSuppressionRequest>,
    ) -> Result<Response<EmailOperatorActionReceipt>, Status> {
        self.record("review", &request);
        Ok(Response::new(receipt("review")))
    }

    async fn get_privacy_activity(
        &self,
        request: Request<GetEmailPrivacyActivityRequest>,
    ) -> Result<Response<EmailPrivacyActivity>, Status> {
        self.record("privacy", &request);
        Ok(Response::new(EmailPrivacyActivity::default()))
    }
}

fn receipt(action_id: &str) -> EmailOperatorActionReceipt {
    EmailOperatorActionReceipt {
        action_id: action_id.into(),
        status: "accepted".into(),
    }
}

async fn spawn_server() -> (String, OperationsService, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let service = OperationsService::default();
    let server_service = service.clone();
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(EmailOperationsServiceServer::new(server_service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("serve operations API");
    });
    (format!("http://{address}"), service, handle)
}

fn client(endpoint: String) -> EmailOperationsClient {
    let config =
        EmailClientConfig::from_values("test", endpoint, TOKEN.into(), Duration::from_millis(500))
            .unwrap();
    EmailOperationsClient {
        inner: EmailOperationsServiceClient::new(config.endpoint.connect_lazy()),
        authorization: config.authorization,
        call_timeout: config.call_timeout,
    }
}

#[tokio::test]
async fn operations_client_exercises_every_rpc_over_real_grpc() {
    let (endpoint, service, server) = spawn_server().await;
    let client = client(endpoint);

    assert_eq!(
        client
            .snapshot(Default::default())
            .await
            .unwrap()
            .queued_message_count,
        7
    );
    assert_eq!(
        client
            .replay_email(Default::default())
            .await
            .unwrap()
            .action_id,
        "replay"
    );
    assert_eq!(
        client
            .apply_suppression(Default::default())
            .await
            .unwrap()
            .action_id,
        "apply"
    );
    assert_eq!(
        client
            .release_suppression(Default::default())
            .await
            .unwrap()
            .action_id,
        "release"
    );
    assert_eq!(
        client
            .review_suppression(Default::default())
            .await
            .unwrap()
            .action_id,
        "review"
    );
    assert!(
        client
            .privacy_activity(Default::default())
            .await
            .unwrap()
            .messages
            .is_empty()
    );

    let calls = service.calls.lock().unwrap();
    assert_eq!(
        calls
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        [
            "snapshot", "replay", "apply", "release", "review", "privacy"
        ]
    );
    assert!(
        calls
            .iter()
            .all(|(_, authorization)| authorization == &format!("Bearer {TOKEN}"))
    );
    drop(calls);
    server.abort();
}

#[test]
fn operations_status_mapping_covers_every_contract_class() {
    let cases = [
        (Code::InvalidArgument, "invalid"),
        (Code::NotFound, "not_found"),
        (Code::FailedPrecondition, "conflict"),
        (Code::AlreadyExists, "conflict"),
        (Code::Unauthenticated, "unauthorized"),
        (Code::PermissionDenied, "unauthorized"),
        (Code::Unavailable, "unavailable"),
        (Code::DeadlineExceeded, "unavailable"),
        (Code::ResourceExhausted, "unavailable"),
        (Code::Internal, "protocol"),
    ];
    for (code, expected) in cases {
        let actual = match map_status(Status::new(code, "detail")) {
            EmailOperationsError::Invalid => "invalid",
            EmailOperationsError::NotFound => "not_found",
            EmailOperationsError::Conflict => "conflict",
            EmailOperationsError::Unauthorized => "unauthorized",
            EmailOperationsError::Unavailable => "unavailable",
            EmailOperationsError::Protocol => "protocol",
            EmailOperationsError::Configuration(_) => "configuration",
        };
        assert_eq!(actual, expected);
    }
}

#[tokio::test]
async fn environment_constructor_has_safe_test_defaults_and_strict_production_config() {
    let _environment = ScopedEnvironment::replace(&[
        ("NVBES_EMAIL_GRPC_ENDPOINT", None),
        ("NVBES_EMAIL_GRPC_AUTH_TOKEN", None),
    ]);
    EmailOperationsClient::from_environment("test").expect("test defaults");
    assert!(matches!(
        EmailOperationsClient::from_environment("production"),
        Err(EmailOperationsError::Configuration(_))
    ));
}
