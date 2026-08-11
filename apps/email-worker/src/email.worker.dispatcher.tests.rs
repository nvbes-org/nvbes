use std::sync::Arc;

use nvbes_email::{EmailError, EmailMessage, EmailSender, SendResult};
use sqlx::PgPool;
use tokio::sync::watch;

use super::{dispatch_message, run_local};
use crate::{crypto::EmailCrypto, database, test_support};

struct FailingSender(EmailError);

#[tonic::async_trait]
impl EmailSender for FailingSender {
    async fn send_message(&self, _message: &EmailMessage) -> Result<SendResult, EmailError> {
        Err(match &self.0 {
            EmailError::Api { status, message } => EmailError::Api {
                status: *status,
                message: message.clone(),
            },
            EmailError::Config(message) => EmailError::Config(message.clone()),
            _ => unreachable!(),
        })
    }
}

async fn accept(pool: &PgPool, key: &str, email: &str) -> uuid::Uuid {
    let command = test_support::command(key, email);
    let receipt = database::accept_command(
        pool,
        &EmailCrypto::new([7; 32], [9; 32]),
        "nvbes.fr",
        &command.clone().into_proto(),
        &command,
    )
    .await
    .unwrap();
    sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
        .bind(receipt.receipt.message_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn dispatch_iteration_materializes_and_delivers_a_persisted_command(pool: PgPool) {
    let message_id = accept(&pool, "dispatch-success", "success@example.com").await;
    let state = test_support::state(pool.clone());
    dispatch_message(&state, message_id).await.unwrap();
    dispatch_message(&state, message_id).await.unwrap();

    let row: (String, Option<String>) =
        sqlx::query_as("SELECT state::text, provider_message_id FROM email_messages WHERE id = $1")
            .bind(message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, "provider_accepted");
    assert!(row.1.is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn dispatch_iteration_classifies_provider_and_stored_payload_failures(pool: PgPool) {
    let transient_id = accept(&pool, "dispatch-transient", "transient@example.com").await;
    let mut transient = test_support::state(pool.clone());
    transient.provider = Arc::new(FailingSender(EmailError::Api {
        status: 503,
        message: "provider details".to_string(),
    }));
    dispatch_message(&transient, transient_id).await.unwrap();
    let transient_state: String =
        sqlx::query_scalar("SELECT state::text FROM email_messages WHERE id = $1")
            .bind(transient_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(transient_state, "deferred");

    sqlx::query(
        "UPDATE email_messages SET next_attempt_at = clock_timestamp() + INTERVAL '1 hour'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let permanent_id = accept(&pool, "dispatch-permanent", "permanent@example.com").await;
    let mut permanent = test_support::state(pool.clone());
    permanent.provider = Arc::new(FailingSender(EmailError::Config("invalid".to_string())));
    dispatch_message(&permanent, permanent_id).await.unwrap();
    let permanent_state: String =
        sqlx::query_scalar("SELECT state::text FROM email_messages WHERE id = $1")
            .bind(permanent_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(permanent_state, "failed");

    let invalid_id = accept(&pool, "dispatch-invalid", "invalid@example.com").await;
    sqlx::query(
        "UPDATE email_messages SET recipient_ciphertext = decode('00', 'hex') WHERE id = $1",
    )
    .bind(invalid_id)
    .execute(&pool)
    .await
    .unwrap();
    dispatch_message(&test_support::state(pool.clone()), invalid_id)
        .await
        .unwrap();
    let invalid_state: String =
        sqlx::query_scalar("SELECT state::text FROM email_messages WHERE id = $1")
            .bind(invalid_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(invalid_state, "failed");
}

#[sqlx::test(migrations = "./migrations")]
async fn local_event_consumer_waits_for_messages_and_stops_cleanly(pool: PgPool) {
    let state = test_support::state(pool);
    let receiver = state.take_local_dispatch_receiver().unwrap();
    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    let dispatcher = tokio::spawn(run_local(state, receiver, shutdown_receiver));
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    shutdown_sender.send(true).unwrap();
    dispatcher.await.unwrap();
}
