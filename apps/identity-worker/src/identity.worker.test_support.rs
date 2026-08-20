use nvbes_redis::{RedisPool, config::RedisConfig};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tonic::{Request, Response, Status, codegen::tokio_stream::Stream, transport::Server};

use nvbes_email::proto::nvbes::email::v1::{
    EmailReceipt as ProtoEmailReceipt, SubmitEmailRequest,
    email_delivery_service_server::{EmailDeliveryService, EmailDeliveryServiceServer},
};

pub async fn database_pool() -> Option<PgPool> {
    let database_url = std::env::var("NVBES_IDENTITY_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/nvbes_identity_test".to_string()
        });
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect_timeout(std::time::Duration::from_millis(500))
        .connect(&database_url)
        .await
        .ok()?;
    if sqlx::migrate!("../identity-service/migrations")
        .run(&pool)
        .await
        .is_err()
    {
        return None;
    }
    Some(pool)
}

pub async fn redis_pool() -> Option<RedisPool> {
    let url =
        std::env::var("NVBES_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    nvbes_redis::connection::create_pool(&RedisConfig {
        url,
        password: std::env::var("NVBES_IDENTITY_WORKER_TEST_REDIS_PASSWORD")
            .or_else(|_| std::env::var("NVBES_REDIS_PASSWORD"))
            .ok(),
        max_connections: 4,
    })
    .await
    .ok()
}

pub async fn app_state() -> Option<crate::app::AppState> {
    let db = database_pool().await?;
    let email_endpoint = spawn_email_service().await;
    set_env("NVBES_EMAIL_GRPC_ENDPOINT", &email_endpoint);
    set_env(
        "NVBES_EMAIL_GRPC_AUTH_TOKEN",
        "identity-worker-email-test-token-0001",
    );
    set_env("NVBES_ACCOUNT_SERVICE_BASE_URL", "http://127.0.0.1:9");
    set_env(
        "NVBES_ACCOUNT_PROVISIONING_TOKEN",
        "identity-worker-account-test-token-01",
    );
    set_env("NVBES_IDENTITY_WORKER_INLINE_HOUSEKEEPING_ENABLED", "false");
    remove_env("NVBES_ENTERPRISE_GRPC_ENDPOINT");
    remove_env("NVBES_ENTERPRISE_GRPC_AUTH_TOKEN");

    let config = nvbes_core::config::AppConfig {
        app_name: "identity-worker-test".to_string(),
        environment: "development".to_string(),
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes_identity_test".to_string()),
        redis_url: std::env::var("NVBES_REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        redis_max_connections: 4,
        web_base_url: "https://account.example.test".to_string(),
        ..nvbes_core::config::AppConfig::default()
    };
    crate::app::AppState::bootstrap(&config, db).await.ok()
}

async fn spawn_email_service() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("email test listener");
    let address = listener.local_addr().expect("email test address");
    tokio::spawn(async move {
        Server::builder()
            .add_service(EmailDeliveryServiceServer::new(TestEmailService))
            .serve_with_incoming(TcpIncoming { listener })
            .await
            .expect("email test service");
    });
    format!("http://{address}")
}

#[derive(Clone)]
struct TestEmailService;

#[tonic::async_trait]
impl EmailDeliveryService for TestEmailService {
    async fn submit_email(
        &self,
        request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoEmailReceipt>, Status> {
        let request = request.into_inner();
        let recipient = request
            .recipient
            .as_ref()
            .map(|recipient| recipient.email.as_str())
            .unwrap_or_default();
        if recipient == "unavailable@example.test" {
            return Err(Status::unavailable("test outage"));
        }
        let accepted_at = chrono::Utc::now();
        let deliver_before = accepted_at + chrono::Duration::minutes(30);
        Ok(Response::new(ProtoEmailReceipt {
            message_id: format!("message-{}", uuid::Uuid::new_v4()),
            accepted_at: Some(timestamp(accepted_at)),
            deliver_before: Some(timestamp(deliver_before)),
            duplicate: false,
        }))
    }
}

fn timestamp(value: chrono::DateTime<chrono::Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

struct TcpIncoming {
    listener: tokio::net::TcpListener,
}

impl Stream for TcpIncoming {
    type Item = Result<tokio::net::TcpStream, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.listener.poll_accept(context) {
            Poll::Ready(Ok((stream, _))) => Poll::Ready(Some(Ok(stream))),
            Poll::Ready(Err(error)) => Poll::Ready(Some(Err(error))),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn set_env(name: &str, value: &str) {
    unsafe { std::env::set_var(name, value) };
}

fn remove_env(name: &str) {
    unsafe { std::env::remove_var(name) };
}
