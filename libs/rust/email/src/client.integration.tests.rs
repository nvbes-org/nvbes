use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{Duration as ChronoDuration, Utc};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Response, Status, transport::Server};

use crate::proto::nvbes::email::v1::{
    EmailReceipt as ProtoEmailReceipt, SubmitEmailRequest,
    email_delivery_service_server::{EmailDeliveryService, EmailDeliveryServiceServer},
};
use crate::{
    EmailCategory, EmailCommand, EmailIdempotencyKey, EmailRecipient, EmailRequestContext,
    EmailTemplate,
};

use super::{EmailClient, EmailClientConfig, EmailClientError};

const TOKEN: &str = "test-email-internal-token-more-than-32-characters";

#[derive(Clone)]
enum Outcome {
    Receipt(ProtoEmailReceipt),
    Status(Code),
}

#[derive(Clone)]
struct DeliveryService {
    outcome: Outcome,
    requests: Arc<Mutex<Vec<Request<SubmitEmailRequest>>>>,
}

#[tonic::async_trait]
impl EmailDeliveryService for DeliveryService {
    async fn submit_email(
        &self,
        request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoEmailReceipt>, Status> {
        self.requests.lock().expect("requests lock").push(request);
        match &self.outcome {
            Outcome::Receipt(receipt) => Ok(Response::new(receipt.clone())),
            Outcome::Status(code) => Err(Status::new(*code, "controlled test failure")),
        }
    }
}

async fn spawn_server(
    outcome: Outcome,
) -> (
    String,
    Arc<Mutex<Vec<Request<SubmitEmailRequest>>>>,
    JoinHandle<()>,
) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind gRPC server");
    let address = listener.local_addr().expect("server address");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let service = DeliveryService {
        outcome,
        requests: Arc::clone(&requests),
    };
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(EmailDeliveryServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("serve gRPC");
    });
    (format!("http://{address}"), requests, handle)
}

fn command() -> EmailCommand {
    let expiry = Utc::now() + ChronoDuration::minutes(10);
    EmailCommand {
        context: EmailRequestContext {
            request_id: "request-1".into(),
            correlation_id: "correlation-1".into(),
            actor_principal_id: "principal-1".into(),
        },
        producer: "identity-service".into(),
        idempotency_key: EmailIdempotencyKey::new("client:e2e:1").unwrap(),
        recipient: EmailRecipient {
            email: "ada@example.com".into(),
            name: Some("Ada".into()),
        },
        category: EmailCategory::Credential,
        template: EmailTemplate::PasswordChangeCodeV1 {
            user_name: "Ada".into(),
            code: "123456".into(),
            credential_expires_at: expiry,
        },
        deliver_before: expiry,
    }
}

fn receipt() -> ProtoEmailReceipt {
    let accepted_at = Utc::now();
    ProtoEmailReceipt {
        message_id: "message-1".into(),
        accepted_at: Some(super::super::command::timestamp(accepted_at)),
        deliver_before: Some(super::super::command::timestamp(
            accepted_at + ChronoDuration::minutes(10),
        )),
        duplicate: false,
    }
}

async fn client(endpoint: String) -> EmailClient {
    let config = EmailClientConfig::from_values(
        "development",
        endpoint,
        TOKEN.into(),
        Duration::from_millis(500),
    )
    .expect("client config");
    EmailClient::connect(config).await.expect("connect client")
}

#[tokio::test]
async fn client_sends_a_validated_authenticated_command_over_real_grpc() {
    let (endpoint, requests, server) = spawn_server(Outcome::Receipt(receipt())).await;
    let client = client(endpoint).await;

    let result = client.send(command()).await.expect("delivery receipt");
    assert_eq!(result.message_id, "message-1");
    assert!(!result.duplicate);

    let requests = requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].metadata().get("authorization").unwrap(),
        &format!("Bearer {TOKEN}")
    );
    assert_eq!(requests[0].get_ref().producer, "identity-service");
    drop(requests);
    server.abort();
}

#[tokio::test]
async fn client_rejects_invalid_commands_before_network_io() {
    let (endpoint, requests, server) = spawn_server(Outcome::Receipt(receipt())).await;
    let client = client(endpoint).await;
    let mut invalid = command();
    invalid.recipient.email = "invalid".into();

    assert!(matches!(
        client.send(invalid).await,
        Err(EmailClientError::InvalidCommand(_))
    ));
    assert!(requests.lock().unwrap().is_empty());
    server.abort();
}

#[tokio::test]
async fn client_maps_every_grpc_failure_class() {
    let cases = [
        (Code::InvalidArgument, "invalid"),
        (Code::AlreadyExists, "conflict"),
        (Code::Unauthenticated, "unauthorized"),
        (Code::PermissionDenied, "unauthorized"),
        (Code::Unavailable, "unavailable"),
        (Code::DeadlineExceeded, "unavailable"),
        (Code::ResourceExhausted, "unavailable"),
        (Code::Internal, "protocol"),
    ];
    for (code, expected) in cases {
        let (endpoint, _, server) = spawn_server(Outcome::Status(code)).await;
        let error = client(endpoint).await.send(command()).await.unwrap_err();
        let actual = match error {
            EmailClientError::InvalidCommand(_) => "invalid",
            EmailClientError::Conflict => "conflict",
            EmailClientError::Unauthorized => "unauthorized",
            EmailClientError::Unavailable => "unavailable",
            EmailClientError::Protocol => "protocol",
            EmailClientError::Configuration(_) => "configuration",
        };
        assert_eq!(actual, expected, "status {code:?}");
        server.abort();
    }
}

#[tokio::test]
async fn client_rejects_each_malformed_receipt_shape() {
    let now = Utc::now();
    let at = |value| Some(super::super::command::timestamp(value));
    let cases = [
        ProtoEmailReceipt {
            message_id: "message".into(),
            accepted_at: None,
            deliver_before: at(now + ChronoDuration::minutes(1)),
            duplicate: false,
        },
        ProtoEmailReceipt {
            message_id: "message".into(),
            accepted_at: at(now),
            deliver_before: None,
            duplicate: false,
        },
        ProtoEmailReceipt {
            message_id: "".into(),
            accepted_at: at(now),
            deliver_before: at(now + ChronoDuration::minutes(1)),
            duplicate: false,
        },
        ProtoEmailReceipt {
            message_id: "message".into(),
            accepted_at: at(now),
            deliver_before: at(now),
            duplicate: false,
        },
        ProtoEmailReceipt {
            message_id: "message".into(),
            accepted_at: Some(prost_types::Timestamp {
                seconds: 0,
                nanos: -1,
            }),
            deliver_before: at(now),
            duplicate: false,
        },
        ProtoEmailReceipt {
            message_id: "message".into(),
            accepted_at: at(now),
            deliver_before: Some(prost_types::Timestamp {
                seconds: 0,
                nanos: -1,
            }),
            duplicate: false,
        },
    ];
    for malformed in cases {
        let (endpoint, _, server) = spawn_server(Outcome::Receipt(malformed)).await;
        assert!(matches!(
            client(endpoint).await.send(command()).await,
            Err(EmailClientError::Protocol)
        ));
        server.abort();
    }
}

#[tokio::test]
async fn connect_classifies_an_unreachable_endpoint_as_unavailable() {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let config = EmailClientConfig::from_values(
        "development",
        endpoint,
        TOKEN.into(),
        Duration::from_millis(100),
    )
    .unwrap();
    assert!(matches!(
        EmailClient::connect(config).await,
        Err(EmailClientError::Unavailable)
    ));
}
