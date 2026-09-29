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
    let mailbox = start_email_fixture().await;
    assert_eq!(
        run_batch(&db, &crypto, &mailbox.client)
            .await
            .unwrap()
            .deferred,
        1
    );
    sqlx::query("UPDATE identity_recovery_deliveries SET available_at=clock_timestamp()")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(
        run_batch(&db, &crypto, &mailbox.client)
            .await
            .unwrap()
            .accepted,
        1
    );
    let captured = mailbox.requests.lock().unwrap().clone();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], captured[1]);
    assert_eq!(captured[0].producer, "identity-service");
    assert_eq!(
        run_batch(&db, &crypto, &mailbox.client)
            .await
            .unwrap()
            .claimed,
        0
    );
    db.close().await;
}

struct ConnectedEmail {
    client: EmailClient,
    requests: Arc<Mutex<Vec<SubmitEmailRequest>>>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for ConnectedEmail {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn start_email_fixture() -> ConnectedEmail {
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
    ConnectedEmail {
        client,
        requests,
        server,
    }
}

#[tokio::test]
async fn password_change_notification_retries_through_email_with_original_recipient() {
    let (db, principal, email) = fixture().await;
    let recovery = crate::recovery::request_recovery(&db, &email)
        .await
        .unwrap();
    crate::recovery::reset_password(&db, &recovery.token, "Password-with-notification!")
        .await
        .unwrap();
    let mailbox = start_email_fixture().await;
    let first = nvbes_identity_service::notification_dispatch::run_batch(&db, &mailbox.client)
        .await
        .unwrap();
    assert_eq!(first.deferred, 1);
    let attempted = mailbox.requests.lock().unwrap()[0].clone();
    let decoded = nvbes_email::EmailCommand::try_from(attempted.clone()).unwrap();
    assert!(matches!(
        decoded.template,
        nvbes_email::EmailTemplate::AccountSecurityV1 {
            event: nvbes_email::AccountSecurityEvent::PasswordRecovered,
            ..
        }
    ));
    let rendered = decoded.template.render();
    assert!(rendered.text_body.contains("password was changed"));
    assert!(!rendered.text_body.contains(&recovery.token));
    sqlx::query("UPDATE identity_login_identifiers SET normalized_value='changed-after-reset@example.invalid' WHERE principal_id=$1").bind(principal).execute(&db).await.unwrap();
    sqlx::query("UPDATE identity_security_notifications SET available_at=clock_timestamp()")
        .execute(&db)
        .await
        .unwrap();
    let resumed = nvbes_identity_service::notification_dispatch::run_batch(&db, &mailbox.client)
        .await
        .unwrap();
    assert_eq!(resumed.accepted, 1);
    let captured = mailbox.requests.lock().unwrap().clone();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], captured[1]);
    assert_eq!(decoded.recipient.email, email);
    let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_security_notifications WHERE command IS NOT NULL OR state<>'accepted'").fetch_one(&db).await.unwrap();
    assert_eq!(retained, 0);
    db.close().await;
}
