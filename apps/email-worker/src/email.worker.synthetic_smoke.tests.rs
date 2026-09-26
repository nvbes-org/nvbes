use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use nvbes_email::{
    EmailCategory, EmailClientConfig, EmailClientError,
    proto::nvbes::email::v1::{
        EmailReceipt as ProtoEmailReceipt, SubmitEmailRequest,
        email_delivery_service_server::{EmailDeliveryService, EmailDeliveryServiceServer},
    },
};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Response, Status, transport::Server};
use uuid::Uuid;

use super::{
    COLD_START_RETRY_SECONDS, DELIVERY_POLL_ATTEMPTS, DELIVERY_POLL_SECONDS,
    MAX_COLD_START_ATTEMPTS, command, is_terminal_failure, required_env, submit_after_cold_start,
    wait_for_signed_delivery_with_pool,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

const GRPC_TOKEN: &str = "test-email-internal-token-more-than-32-characters";

#[test]
fn builds_an_explicit_bounded_operational_message() {
    let now = Utc.with_ymd_and_hms(2026, 8, 25, 20, 0, 0).unwrap();
    let command = command("operator@example.com", "deploy-123", now).unwrap();

    assert_eq!(command.category, EmailCategory::Operational);
    assert_eq!(command.deliver_before, now + ChronoDuration::minutes(30));
    let rendered = command.template.render();
    assert!(rendered.subject.contains("no action required"));
    assert!(rendered.text_body.contains("deploy-123"));
    assert!(!rendered.text_body.contains("operator@example.com"));
}

#[test]
fn rejects_an_unbounded_or_unsafe_run_identifier() {
    let now = Utc.with_ymd_and_hms(2026, 8, 25, 20, 0, 0).unwrap();
    assert!(command("operator@example.com", "contains spaces", now).is_err());
}

#[test]
fn cold_start_retry_schedule_is_bounded() {
    assert_eq!(MAX_COLD_START_ATTEMPTS, 5);
    assert_eq!(COLD_START_RETRY_SECONDS, [1, 2, 4, 8]);
    assert_eq!(COLD_START_RETRY_SECONDS.iter().sum::<u64>(), 15);
}

#[test]
fn delivery_poll_is_bounded_and_rejects_terminal_failures() {
    assert_eq!(DELIVERY_POLL_ATTEMPTS, 60);
    assert_eq!(DELIVERY_POLL_SECONDS, 5);
    for state in [
        "hard_bounced",
        "complained",
        "suppressed",
        "expired",
        "dropped",
        "failed",
    ] {
        assert!(is_terminal_failure(state), "{state}");
    }
    assert!(!is_terminal_failure("provider_accepted"));
    assert!(!is_terminal_failure("delivered"));
}

#[test]
fn required_env_rejects_missing_and_blank_values() {
    let _lock = ENV_LOCK.lock().unwrap();
    let name = "NVBES_EMAIL_SYNTHETIC_TEST_ONLY_VAR";
    let previous = std::env::var_os(name);
    unsafe { std::env::remove_var(name) };
    assert!(required_env(name).is_err());
    unsafe { std::env::set_var(name, "   ") };
    assert!(required_env(name).is_err());
    unsafe { std::env::set_var(name, "ok-value") };
    assert_eq!(required_env(name).unwrap(), "ok-value");
    match previous {
        Some(value) => unsafe { std::env::set_var(name, value) },
        None => unsafe { std::env::remove_var(name) },
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn wait_for_signed_delivery_accepts_delivered_message_with_provider_event(
    pool: sqlx::PgPool,
) {
    let message_id = format!("msg-{}", Uuid::new_v4());
    seed_message(&pool, &message_id, "delivered", true).await;

    let (state, events) = wait_for_signed_delivery_with_pool(&pool, &message_id)
        .await
        .expect("delivered");
    assert_eq!(state, "delivered");
    assert!(events > 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn wait_for_signed_delivery_rejects_terminal_failure_states(pool: sqlx::PgPool) {
    let message_id = format!("msg-{}", Uuid::new_v4());
    seed_message(&pool, &message_id, "failed", false).await;

    let error = wait_for_signed_delivery_with_pool(&pool, &message_id)
        .await
        .expect_err("terminal");
    assert!(
        error
            .to_string()
            .contains("synthetic email reached terminal state failed"),
        "{error}"
    );
}

#[tokio::test]
async fn submit_after_cold_start_retries_unavailable_then_succeeds() {
    let (endpoint, server) = spawn_flaky_delivery_server().await;
    let config = EmailClientConfig::from_values(
        "development",
        endpoint,
        GRPC_TOKEN.into(),
        Duration::from_millis(500),
    )
    .expect("config");
    let now = Utc::now();
    let command = command("operator@example.com", "cold-start-1", now).expect("command");
    let (receipt, attempts) = submit_after_cold_start(&config, &command)
        .await
        .expect("receipt after retry");
    assert_eq!(receipt.message_id, "message-1");
    assert_eq!(attempts, 2);
    server.abort();
}

#[tokio::test]
async fn submit_after_cold_start_fails_fast_on_non_retryable_errors() {
    let (endpoint, server) = spawn_status_server(Code::InvalidArgument).await;
    let config = EmailClientConfig::from_values(
        "development",
        endpoint,
        GRPC_TOKEN.into(),
        Duration::from_millis(500),
    )
    .expect("config");
    let now = Utc::now();
    let command = command("operator@example.com", "cold-start-2", now).expect("command");
    let error = submit_after_cold_start(&config, &command)
        .await
        .expect_err("non-retryable");
    assert!(matches!(error, EmailClientError::InvalidCommand(_)));
    server.abort();
}

async fn seed_message(pool: &sqlx::PgPool, message_id: &str, state: &str, with_event: bool) {
    let id = Uuid::new_v4();
    let provider_message_id = format!("provider-{message_id}");
    sqlx::query(
        r#"
        INSERT INTO email_messages (
            id, producer, idempotency_key, command_fingerprint, request_id, correlation_id,
            category, template_name, template_version, recipient_hash, recipient_ciphertext,
            recipient_nonce, template_ciphertext, template_nonce, deliver_before, state,
            message_id, provider_message_id
        ) VALUES (
            $1, 'readiness', $2, $3, 'request', 'correlation', 'credential', 'template', 1,
            $4, $5, $6, $7, $8, now() + interval '30 minutes', $9::email_message_state,
            $10, $11
        )
        "#,
    )
    .bind(id)
    .bind(format!("idem-{message_id}"))
    .bind(vec![3_u8; 32])
    .bind(vec![4_u8; 32])
    .bind(vec![5_u8; 16])
    .bind(vec![6_u8; 12])
    .bind(vec![7_u8; 16])
    .bind(vec![8_u8; 12])
    .bind(state)
    .bind(message_id)
    .bind(&provider_message_id)
    .execute(pool)
    .await
    .expect("seed message");

    if with_event {
        sqlx::query(
            r#"
            INSERT INTO email_provider_events (
                id, provider, provider_event_id, sns_message_id, provider_message_id,
                event_type, processed_at, processing_result
            ) VALUES ($1, 'scaleway', $2, $3, $4, 'delivery', now(), 'state_applied')
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(format!("event-{message_id}"))
        .bind(format!("sns-{message_id}"))
        .bind(provider_message_id)
        .execute(pool)
        .await
        .expect("seed event");
    }
}

#[derive(Clone)]
struct FlakyDelivery {
    fail_count: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl EmailDeliveryService for FlakyDelivery {
    async fn submit_email(
        &self,
        _request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoEmailReceipt>, Status> {
        let prior = self.fail_count.fetch_add(1, Ordering::SeqCst);
        if prior < 1 {
            return Err(Status::new(Code::Unavailable, "cold start"));
        }
        let accepted_at = Utc::now();
        Ok(Response::new(ProtoEmailReceipt {
            message_id: "message-1".into(),
            accepted_at: Some(prost_timestamp(accepted_at)),
            deliver_before: Some(prost_timestamp(accepted_at + ChronoDuration::minutes(10))),
            duplicate: false,
        }))
    }
}

fn prost_timestamp(value: chrono::DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

#[derive(Clone)]
struct StatusDelivery {
    code: Code,
}

#[tonic::async_trait]
impl EmailDeliveryService for StatusDelivery {
    async fn submit_email(
        &self,
        _request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoEmailReceipt>, Status> {
        Err(Status::new(self.code, "controlled"))
    }
}

async fn spawn_flaky_delivery_server() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind gRPC");
    let address = listener.local_addr().expect("addr");
    let service = FlakyDelivery {
        fail_count: Arc::new(AtomicUsize::new(0)),
    };
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(EmailDeliveryServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("serve");
    });
    (format!("http://{address}"), handle)
}

async fn spawn_status_server(code: Code) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind gRPC");
    let address = listener.local_addr().expect("addr");
    let service = StatusDelivery { code };
    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(EmailDeliveryServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("serve");
    });
    (format!("http://{address}"), handle)
}
