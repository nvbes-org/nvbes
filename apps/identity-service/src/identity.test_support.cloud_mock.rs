use std::pin::Pin;
use std::task::{Context, Poll};

use sqlx::PgPool;
use tonic::codegen::tokio_stream::Stream;
use tonic::transport::Server;

use crate::grpc_pb::nvbes::cloud::v1 as cloud;
use cloud::cloud_service_server::CloudServiceServer;

#[path = "identity.test_support.cloud_mock.queries.rs"]
mod queries;
#[path = "identity.test_support.cloud_mock.service.rs"]
mod service;

static CLOUD_MOCK: std::sync::OnceLock<RunningCloudMock> = std::sync::OnceLock::new();

struct RunningCloudMock {
    endpoint: String,
    thread: std::thread::JoinHandle<()>,
}

#[derive(Clone)]
struct AccountCloudMock {
    db: PgPool,
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

pub(crate) fn ensure_cloud_mock(db: &PgPool) {
    let running = CLOUD_MOCK.get_or_init(|| start_cloud_mock(db.clone()));
    assert!(
        !running.thread.is_finished(),
        "Account Cloud gRPC test server must remain available"
    );

    unsafe {
        std::env::set_var("NVBES_CLOUD_GRPC_ENDPOINT", &running.endpoint);
    }
}

fn start_cloud_mock(db: PgPool) -> RunningCloudMock {
    let (endpoint_sender, endpoint_receiver) = std::sync::mpsc::sync_channel(1);
    let thread = std::thread::Builder::new()
        .name("oauth-cloud-grpc-test".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Account Cloud gRPC test runtime must start");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("Account Cloud gRPC test server must bind an ephemeral port");
                let address = listener
                    .local_addr()
                    .expect("Account Cloud gRPC test server address must be available");
                endpoint_sender
                    .send(format!("http://{address}"))
                    .expect("Account Cloud gRPC test endpoint must reach the test thread");

                Server::builder()
                    .add_service(CloudServiceServer::new(AccountCloudMock { db }))
                    .serve_with_incoming(TcpIncoming { listener })
                    .await
                    .expect("Account Cloud gRPC test server must remain healthy");
            });
        })
        .expect("Account Cloud gRPC test thread must start");
    let endpoint = endpoint_receiver
        .recv()
        .expect("Account Cloud gRPC test endpoint must be available");

    RunningCloudMock { endpoint, thread }
}
