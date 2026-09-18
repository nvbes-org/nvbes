use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use nvbes_email::{
    EmailCategory, EmailClient, EmailClientConfig, EmailClientError, EmailCommand,
    EmailIdempotencyKey, EmailRecipient, EmailRequestContext, EmailTemplate,
};
use sqlx::PgPool;
use tokio::net::TcpListener;

use super::{EmailDeliveryGrpcService, delivery_server};
use crate::test_support;

async fn spawn_provider(pool: PgPool) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind provider listener");
    let address = listener.local_addr().expect("provider address");
    let service = EmailDeliveryGrpcService::new(test_support::state(pool));
    let app = tonic::service::Routes::new(delivery_server(service)).into_axum_router();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve provider");
    });
    (format!("http://{address}"), server)
}

fn client_config(endpoint: String, token: &str) -> EmailClientConfig {
    EmailClientConfig::from_values("test", endpoint, token.to_string(), Duration::from_secs(5))
        .expect("valid client configuration")
}

fn identity_recovery_command(idempotency_key: &str) -> EmailCommand {
    let expiry = Utc::now() + ChronoDuration::minutes(15);
    EmailCommand {
        context: EmailRequestContext {
            request_id: format!("req-{idempotency_key}"),
            correlation_id: format!("corr-{idempotency_key}"),
            actor_principal_id: "principal-user-123".to_string(),
        },
        producer: "identity-service".to_string(),
        idempotency_key: EmailIdempotencyKey::new(format!("identity:recovery:{idempotency_key}"))
            .expect("valid idempotency key"),
        recipient: EmailRecipient {
            email: "user@example.com".to_string(),
            name: Some("Target User".to_string()),
        },
        category: EmailCategory::Credential,
        template: EmailTemplate::PasswordResetV1 {
            user_name: "Target User".to_string(),
            reset_url: "https://identity.nvbes.fr/recover".to_string(),
            credential_expires_at: expiry,
        },
        deliver_before: expiry,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn consumer_contract_delivers_command_and_receives_verified_receipt(pool: PgPool) {
    let (url, server) = spawn_provider(pool).await;
    let client = EmailClient::connect(client_config(url, test_support::INTERNAL_TOKEN))
        .await
        .expect("consumer connects to provider");

    let command = identity_recovery_command("first-dispatch");
    let receipt = client
        .send(command.clone())
        .await
        .expect("consumer sends command successfully");

    assert!(!receipt.message_id.is_empty());
    assert!(!receipt.duplicate);
    assert!(receipt.deliver_before > receipt.accepted_at);

    // Contract: Idempotent replay returns same receipt marked duplicate
    let duplicate_receipt = client
        .send(command)
        .await
        .expect("consumer replays command idempotently");

    assert_eq!(duplicate_receipt.message_id, receipt.message_id);
    assert!(duplicate_receipt.duplicate);

    server.abort();
}

#[sqlx::test(migrations = "./migrations")]
async fn consumer_contract_maps_conflicts_and_unauthorized_errors(pool: PgPool) {
    let (url, server) = spawn_provider(pool).await;
    let authorized_client =
        EmailClient::connect(client_config(url.clone(), test_support::INTERNAL_TOKEN))
            .await
            .expect("consumer connects with valid token");

    let original = identity_recovery_command("conflict-key");
    authorized_client
        .send(original.clone())
        .await
        .expect("initial send succeeds");

    // Contract: Idempotency conflict returns EmailClientError::Conflict
    let mut conflicting = original;
    conflicting.recipient.email = "other-recipient@example.com".to_string();
    let conflict_err = authorized_client
        .send(conflicting)
        .await
        .expect_err("conflicting command must fail");

    assert!(matches!(conflict_err, EmailClientError::Conflict));

    // Contract: Invalid token returns EmailClientError::Unauthorized
    let unauthorized_token = "invalid-token-with-sufficient-length-32-chars";
    let unauthorized_client = EmailClient::connect(client_config(url, unauthorized_token))
        .await
        .expect("client connects");

    let auth_err = unauthorized_client
        .send(identity_recovery_command("unauthorized-key"))
        .await
        .expect_err("unauthorized send must fail");

    assert!(matches!(auth_err, EmailClientError::Unauthorized));

    server.abort();
}
