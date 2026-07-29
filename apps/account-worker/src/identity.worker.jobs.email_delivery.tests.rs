use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use nvbes_email::{EmailAddress, EmailError, EmailMessage, EmailSender, SendResult};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{deliver_email, deterministic_message_id};

#[derive(Debug, Clone, Copy)]
enum FakeResponse {
    Success,
    TransientFailure,
    PermanentFailure,
}

struct CountingSender {
    sends: AtomicUsize,
    responses: Mutex<VecDeque<FakeResponse>>,
    message_ids: Mutex<Vec<String>>,
    delay: Duration,
}

impl CountingSender {
    fn new(responses: impl IntoIterator<Item = FakeResponse>) -> Self {
        Self {
            sends: AtomicUsize::new(0),
            responses: Mutex::new(responses.into_iter().collect()),
            message_ids: Mutex::new(Vec::new()),
            delay: Duration::ZERO,
        }
    }

    fn delayed(delay: Duration) -> Self {
        Self {
            delay,
            ..Self::new([FakeResponse::Success])
        }
    }

    fn send_count(&self) -> usize {
        self.sends.load(Ordering::SeqCst)
    }

    async fn captured_message_ids(&self) -> Vec<String> {
        self.message_ids.lock().await.clone()
    }
}

#[async_trait]
impl EmailSender for CountingSender {
    async fn send_message(&self, message: &EmailMessage) -> Result<SendResult, EmailError> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        let message_id = message
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("message-id"))
            .map(|(_, value)| value.clone())
            .expect("delivery must attach Message-ID");
        assert!(
            message.headers.iter().any(|(name, value)| name
                .eq_ignore_ascii_case("x-nvbes-email-job-id")
                && message_id.contains(value)),
            "delivery must attach the matching job id"
        );
        self.message_ids.lock().await.push(message_id);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }

        match self
            .responses
            .lock()
            .await
            .pop_front()
            .unwrap_or(FakeResponse::Success)
        {
            FakeResponse::Success => Ok(SendResult {
                provider_email_id: format!("provider-{}", self.send_count()),
            }),
            FakeResponse::TransientFailure => Err(EmailError::Api {
                status: 503,
                message: "provider secret detail".to_string(),
            }),
            FakeResponse::PermanentFailure => Err(EmailError::Api {
                status: 422,
                message: "recipient rejected with private detail".to_string(),
            }),
        }
    }
}

fn message() -> EmailMessage {
    EmailMessage {
        from: EmailAddress {
            email: "sender@example.test".to_string(),
            name: Some("nvbes".to_string()),
        },
        to: vec![EmailAddress {
            email: "recipient@example.test".to_string(),
            name: Some("Recipient".to_string()),
        }],
        subject: "Account event".to_string(),
        text_body: Some("Account event".to_string()),
        html_body: Some("<p>Account event</p>".to_string()),
        headers: Vec::new(),
    }
}

async fn deliver(
    pool: &PgPool,
    sender: &CountingSender,
    job_id: Uuid,
) -> Result<super::EmailDeliveryOutcome, crate::worker::job_failure::JobExecutionError> {
    deliver_email(
        pool,
        sender,
        job_id,
        "data_export",
        "recipient@example.test",
        "Account event",
        message(),
    )
    .await
}

#[sqlx::test(migrations = "../account-service/migrations")]
async fn already_sent_job_is_deduplicated(pool: PgPool) {
    let sender = CountingSender::new([FakeResponse::Success]);
    let job_id = Uuid::new_v4();

    let first = deliver(&pool, &sender, job_id)
        .await
        .expect("first delivery should succeed");
    let second = deliver(&pool, &sender, job_id)
        .await
        .expect("retry should read the completed ledger");

    assert!(!first.deduplicated);
    assert!(second.deduplicated);
    assert_eq!(sender.send_count(), 1);
    assert_eq!(
        sender.captured_message_ids().await,
        vec![deterministic_message_id(job_id)]
    );

    let collision = deliver_email(
        &pool,
        &sender,
        job_id,
        "data_export",
        "different-recipient@example.test",
        "Account event",
        message(),
    )
    .await
    .expect_err("one job id must not be rebound to another recipient");
    assert!(!collision.is_retryable());
    assert_eq!(collision.code(), "email_job_identity_conflict");
    assert_eq!(sender.send_count(), 1);
}

#[sqlx::test(migrations = "../account-service/migrations")]
async fn transient_failure_retries_but_permanent_failure_does_not(pool: PgPool) {
    let transient = CountingSender::new([FakeResponse::TransientFailure, FakeResponse::Success]);
    let transient_job = Uuid::new_v4();
    let first = deliver(&pool, &transient, transient_job)
        .await
        .expect_err("transient provider failure should fail the attempt");
    assert!(first.is_retryable());
    deliver(&pool, &transient, transient_job)
        .await
        .expect("transient provider failure should be retried");
    assert_eq!(transient.send_count(), 2);
    let transient_ledger = sqlx::query_as::<_, (String, i32, Option<String>, Option<String>)>(
        r#"
            SELECT status::text, attempt_count, last_error_class, last_error_summary
            FROM email_messages
            WHERE job_id = $1
            "#,
    )
    .bind(transient_job)
    .fetch_one(&pool)
    .await
    .expect("transient ledger should exist");
    assert_eq!(transient_ledger, ("sent".to_string(), 2, None, None));

    let permanent = CountingSender::new([FakeResponse::PermanentFailure]);
    let permanent_job = Uuid::new_v4();
    let first = deliver(&pool, &permanent, permanent_job)
        .await
        .expect_err("permanent provider failure should fail the job");
    let replay = deliver(&pool, &permanent, permanent_job)
        .await
        .expect_err("durable permanent failure should be replayed");
    assert!(!first.is_retryable());
    assert!(!replay.is_retryable());
    assert_eq!(permanent.send_count(), 1);
    assert!(!replay.to_string().contains("private detail"));
    let permanent_ledger = sqlx::query_as::<_, (String, i32, Option<String>, Option<String>)>(
        r#"
            SELECT status::text, attempt_count, last_error_class, last_error_summary
            FROM email_messages
            WHERE job_id = $1
            "#,
    )
    .bind(permanent_job)
    .fetch_one(&pool)
    .await
    .expect("permanent ledger should exist");
    assert_eq!(
        permanent_ledger,
        (
            "failed".to_string(),
            1,
            Some("permanent".to_string()),
            Some("Email provider rejected the request".to_string()),
        )
    );
}

#[sqlx::test(migrations = "../account-service/migrations")]
async fn concurrent_claims_send_only_once(pool: PgPool) {
    let sender = Arc::new(CountingSender::delayed(Duration::from_millis(100)));
    let job_id = Uuid::new_v4();
    let first_sender = Arc::clone(&sender);
    let second_sender = Arc::clone(&sender);

    let (first, second) = tokio::join!(
        deliver(&pool, first_sender.as_ref(), job_id),
        deliver(&pool, second_sender.as_ref(), job_id),
    );

    assert!(first.is_ok());
    assert!(second.is_ok());
    assert_eq!(sender.send_count(), 1);
    assert!(
        first.expect("first result").deduplicated ^ second.expect("second result").deduplicated
    );
}

#[sqlx::test(migrations = "../account-service/migrations")]
async fn post_send_commit_failure_reuses_message_id_on_retry(pool: PgPool) {
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION fail_email_sent_event() RETURNS trigger AS $$
        BEGIN
            IF NEW.event_type = 'email_sent' THEN
                RAISE EXCEPTION 'injected commit-path failure';
            END IF;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        CREATE CONSTRAINT TRIGGER email_sent_failure
        AFTER INSERT ON email_events
        DEFERRABLE INITIALLY DEFERRED
        FOR EACH ROW EXECUTE FUNCTION fail_email_sent_event();
        "#,
    )
    .execute(&pool)
    .await
    .expect("failure trigger should install");

    let sender = CountingSender::new([FakeResponse::Success, FakeResponse::Success]);
    let job_id = Uuid::new_v4();
    let first = deliver(&pool, &sender, job_id)
        .await
        .expect_err("database failure after provider acceptance should surface");
    assert!(first.is_retryable());

    sqlx::raw_sql(
        "DROP TRIGGER email_sent_failure ON email_events; DROP FUNCTION fail_email_sent_event();",
    )
    .execute(&pool)
    .await
    .expect("failure trigger should be removed");
    deliver(&pool, &sender, job_id)
        .await
        .expect("retry should complete");

    assert_eq!(sender.send_count(), 2);
    assert_eq!(
        sender.captured_message_ids().await,
        vec![
            deterministic_message_id(job_id),
            deterministic_message_id(job_id)
        ]
    );
}
