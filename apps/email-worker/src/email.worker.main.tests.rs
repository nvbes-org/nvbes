use std::path::PathBuf;

use tokio::sync::watch;

use super::{provider_name, redacted_sender, server_shutdown};
use crate::config::{ProviderConfig, ScalewayConfig};

#[test]
fn startup_log_values_are_bounded_and_sender_is_redacted() {
    assert_eq!(provider_name(&ProviderConfig::Mock), "mock");
    assert_eq!(
        provider_name(&ProviderConfig::TestCapture(PathBuf::from("/tmp/capture"))),
        "test-capture"
    );
    assert_eq!(
        provider_name(&ProviderConfig::Scaleway(ScalewayConfig {
            secret_key: "secret".to_string(),
            project_id: "project".to_string(),
            region: "fr-par".to_string(),
        })),
        "scaleway"
    );
    assert_eq!(redacted_sender("no-reply@nvbes.fr"), "***@nvbes.fr");
    assert_eq!(redacted_sender("invalid"), "***");
}

#[tokio::test]
async fn server_shutdown_completes_on_signal_or_closed_channel() {
    let (sender, receiver) = watch::channel(false);
    let waiting = tokio::spawn(server_shutdown(receiver));
    sender.send(true).unwrap();
    waiting.await.unwrap();

    let (sender, receiver) = watch::channel(false);
    drop(sender);
    server_shutdown(receiver).await;
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn audited_release_command_validates_input_and_changes_suppression(pool: sqlx::PgPool) {
    use crate::{database, operations_actions};

    let email = "release-cli@example.com";
    operations_actions::apply_suppression(
        &pool,
        &crate::crypto::EmailCrypto::new([7; 32], [9; 32]),
        email,
        "all",
        operations_actions::OperatorAction {
            actor: "00000000-0000-0000-0000-000000000001",
            reason: "ticket EMAIL-123 approved",
        },
    )
    .await
    .unwrap();
    let command = crate::test_support::command("release-cli", email);
    let receipt = database::accept_command(
        &pool,
        &crate::crypto::EmailCrypto::new([7; 32], [9; 32]),
        "nvbes.fr",
        &command.clone().into_proto(),
        &command,
    )
    .await
    .unwrap();
    let message_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(receipt.receipt.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(
        super::release_suppression(&pool, "invalid", "actor", "valid reason")
            .await
            .is_err()
    );
    assert!(
        super::release_suppression(&pool, &message_id.to_string(), "bad actor", "valid reason")
            .await
            .is_err()
    );
    assert!(
        super::release_suppression(&pool, &message_id.to_string(), "operator:1", "bad\nreason")
            .await
            .is_err()
    );
    super::release_suppression(
        &pool,
        &message_id.to_string(),
        "operator:1",
        "recipient ownership verified",
    )
    .await
    .unwrap();
    assert!(
        super::release_suppression(
            &pool,
            &message_id.to_string(),
            "operator:1",
            "recipient ownership verified",
        )
        .await
        .is_err()
    );
}
