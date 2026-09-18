use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use nvbes_email::proto::nvbes::email::v1::{
    EmailReceipt, SubmitEmailRequest,
    email_delivery_service_server::{EmailDeliveryService, EmailDeliveryServiceServer},
    transactional_email_template::Template,
};
use nvbes_email::{EmailClient, EmailClientConfig};
use openssl::{pkey::PKey, rsa::Rsa};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status, transport::Server};
use uuid::Uuid;

use crate::{
    auth::RecoveryNotification,
    email::deliver_recovery,
    mfa,
    mfa_crypto::MfaCrypto,
    mfa_rotation,
    synthetic::{
        run as run_synthetic_smoke, run_with_delivery as run_synthetic_smoke_with_delivery,
    },
    tokens::TokenService,
    tokens_config::TokenConfig,
    tokens_synthetic::run_synthetic_smoke as run_synthetic_token_smoke,
};

#[sqlx::test(migrations = "./migrations")]
async fn synthetic_identity_authentication_and_recovery_are_transactional(pool: PgPool) {
    let email = format!("synthetic-{}@example.invalid", Uuid::new_v4());

    let result = run_synthetic_smoke(
        &pool,
        &email,
        "Initial-database-test-password!",
        "Recovered-database-test-password!",
    )
    .await
    .expect("synthetic lifecycle succeeds");

    assert!(result.session_rotated);
    assert!(result.recovery_consumed);
    assert_eq!(result.audit_events, 5);
}

#[derive(Clone, Default)]
struct CapturingEmailService {
    requests: Arc<Mutex<Vec<Request<SubmitEmailRequest>>>>,
}

#[tonic::async_trait]
impl EmailDeliveryService for CapturingEmailService {
    async fn submit_email(
        &self,
        request: Request<SubmitEmailRequest>,
    ) -> Result<Response<EmailReceipt>, Status> {
        self.requests.lock().unwrap().push(request);
        let accepted_at = Utc::now();
        Ok(Response::new(EmailReceipt {
            message_id: Uuid::new_v4().to_string(),
            accepted_at: Some(timestamp(accepted_at)),
            deliver_before: Some(timestamp(accepted_at + Duration::minutes(15))),
            duplicate: false,
        }))
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn recovery_is_accepted_by_email_before_it_is_consumed(pool: PgPool) {
    let service = CapturingEmailService::default();
    let requests = Arc::clone(&service.requests);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(EmailDeliveryServiceServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .unwrap();
    });
    let email_client = EmailClient::connect(
        EmailClientConfig::from_values(
            "test",
            format!("http://{address}"),
            "identity-email-test-token-at-least-32-characters".into(),
            std::time::Duration::from_secs(2),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let email = format!("synthetic-{}@example.invalid", Uuid::new_v4());

    let result = run_synthetic_smoke_with_delivery(
        &pool,
        &email,
        "Initial-email-test-password!",
        "Recovered-email-test-password!",
        |recovery: RecoveryNotification| {
            deliver_recovery(&email_client, "https://identity.nvbes.eu/recover", recovery)
        },
    )
    .await
    .expect("integrated recovery succeeds");

    assert!(result.recovery_consumed);
    let captured = requests.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].get_ref().producer, "identity-service");
    assert!(matches!(
        captured[0]
            .get_ref()
            .template
            .as_ref()
            .and_then(|template| template.template.as_ref()),
        Some(Template::PasswordResetV1(_))
    ));
    assert_eq!(
        captured[0].metadata().get("authorization").unwrap(),
        "Bearer identity-email-test-token-at-least-32-characters"
    );
    drop(captured);
    server.abort();
}

#[sqlx::test(migrations = "./migrations")]
async fn totp_step_up_is_encrypted_audited_and_replay_safe(pool: PgPool) {
    let email = format!("synthetic-mfa-{}@example.invalid", Uuid::new_v4());

    let result = mfa::run_synthetic_smoke(
        &pool,
        &MfaCrypto::with_rotation(1, [11; 32], None).unwrap(),
        &email,
        "Synthetic-mfa-test-password!",
    )
    .await
    .expect("MFA lifecycle succeeds");

    assert!(result.factor_encrypted);
    assert!(result.enrollment_confirmed);
    assert!(result.step_up_granted);
    assert!(result.replay_rejected);
    assert_eq!(result.audit_events, 5);

    let rotating = MfaCrypto::with_rotation(2, [12; 32], Some((1, [11; 32]))).unwrap();
    assert_eq!(mfa_rotation::rotate(&pool, &rotating).await.unwrap(), 1);
    let factor: (Uuid, Vec<u8>, Vec<u8>, i16) = sqlx::query_as(
        "SELECT id, secret_ciphertext, secret_nonce, key_version FROM identity_auth_factors WHERE principal_id = $1",
    )
    .bind(result.principal_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(factor.3, 2);
    assert!(
        !rotating
            .open(factor.0, factor.3, &factor.1, &factor.2)
            .unwrap()
            .is_empty()
    );
    let rotation_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity_audit_events WHERE principal_id = $1 AND event_type = 'identity.mfa_key_rotated'",
    )
    .bind(result.principal_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rotation_audits, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn access_token_introspection_tracks_session_revocation(pool: PgPool) {
    let private = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let service = TokenService::new(
        TokenConfig::from_values(
            "test",
            "http://localhost:3000".into(),
            "identity-key-1".into(),
            String::from_utf8(private.private_key_to_pem_pkcs8().unwrap()).unwrap(),
            String::from_utf8(private.public_key_to_pem().unwrap()).unwrap(),
            "nvbes-account-service".into(),
        )
        .unwrap(),
    )
    .unwrap();
    let email = format!("synthetic-token-{}@example.invalid", Uuid::new_v4());

    let result = run_synthetic_token_smoke(
        &pool,
        &service,
        &email,
        "Synthetic-token-test-password!",
        "nvbes-account-service",
    )
    .await
    .expect("token lifecycle succeeds");

    assert_eq!(result.algorithm, "RS256");
    assert_eq!(result.expires_in_seconds, 900);
    assert_eq!(
        result
            .scope
            .split_whitespace()
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "account:read",
            "account:write",
            "account:export",
            "account:close"
        ])
    );
    assert_eq!(result.amr, ["pwd"]);
    assert!(result.active_before_revocation);
    assert!(result.inactive_for_wrong_audience);
    assert!(result.inactive_after_revocation);
    let audit_mutation = sqlx::query("DELETE FROM identity_audit_events WHERE principal_id=$1")
        .bind(result.principal_id)
        .execute(&pool)
        .await;
    assert!(
        audit_mutation.is_err(),
        "Identity audit events are append-only"
    );
}

fn timestamp(value: chrono::DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}
