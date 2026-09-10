use super::*;
use crate::recovery_test_fixture::fixture;
use nvbes_email::proto::nvbes::email::v1::{
    EmailReceipt as ProtoReceipt, SubmitEmailRequest,
    email_delivery_service_server::{EmailDeliveryService, EmailDeliveryServiceServer},
};
use std::sync::{Arc, Mutex};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status};

#[derive(Clone, Default)]
struct EmailFixture {
    requests: Arc<Mutex<Vec<SubmitEmailRequest>>>,
}

#[tonic::async_trait]
impl EmailDeliveryService for EmailFixture {
    async fn submit_email(
        &self,
        request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoReceipt>, Status> {
        assert_eq!(
            request.metadata().get("authorization").unwrap(),
            "Bearer recovery-delivery-test-token-at-least-32-characters"
        );
        let command = request.into_inner();
        let mut requests = self.requests.lock().unwrap();
        requests.push(command.clone());
        if requests.len() == 1 {
            return Err(Status::unavailable("synthetic outage"));
        }
        let now = chrono::Utc::now();
        Ok(Response::new(ProtoReceipt {
            message_id: Uuid::new_v4().to_string(),
            accepted_at: Some(prost_types::Timestamp {
                seconds: now.timestamp(),
                nanos: now.timestamp_subsec_nanos() as i32,
            }),
            deliver_before: command.deliver_before,
            duplicate: false,
        }))
    }
}

#[tokio::test]
async fn encrypted_recovery_survives_real_email_client_transport_failure() {
    let (db, _, email) = fixture().await;
    let crypto = MfaCrypto::with_rotation(1, [42; 32], None).unwrap();
    enqueue(&db, &crypto, "https://identity.example/recover", &email)
        .await
        .unwrap();
    let service = EmailFixture::default();
    let requests = service.requests.clone();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(EmailDeliveryServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });
    let client = EmailClient::connect(
        nvbes_email::EmailClientConfig::from_values(
            "test",
            format!("http://{address}"),
            "recovery-delivery-test-token-at-least-32-characters".into(),
            std::time::Duration::from_secs(2),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(run_batch(&db, &crypto, &client).await.unwrap().deferred, 1);
    sqlx::query("UPDATE identity_recovery_deliveries SET available_at=clock_timestamp()")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(run_batch(&db, &crypto, &client).await.unwrap().accepted, 1);
    let captured = requests.lock().unwrap().clone();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], captured[1]);
    assert_eq!(captured[0].producer, "identity-service");
    assert_eq!(run_batch(&db, &crypto, &client).await.unwrap().claimed, 0);
    server.abort();
    db.close().await;
}
