use chrono::Utc;
use nvbes_email::{EmailClient, EmailClientConfig, EmailCommand};
use sqlx::PgPool;
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};
use uuid::Uuid;

const TOKEN: &str = "identity-email-runtime-test-token-32-value";

struct Worker {
    child: Child,
    command: Command,
    captures: PathBuf,
    endpoint: String,
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.captures);
    }
}

impl Worker {
    async fn start(url: &str) -> Self {
        let binary = std::env::var("NVBES_IDENTITY_TEST_EMAIL_BINARY")
            .expect("run the identity-service:test:email-runtime target");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let captures = std::env::temp_dir().join(format!("identity-email-{}", Uuid::new_v4()));
        let mut command = Command::new(binary);
        // Never inherit real provider credentials, telemetry or queue configuration.
        command
            .env_clear()
            .env("NVBES_ENVIRONMENT", "test")
            .env(
                "NVBES_OBSERVABILITY_INTERNAL_TOKEN",
                "email-observability-test-token-32-value",
            )
            .env("NVBES_EMAIL_DATABASE_URL", url)
            .env("NVBES_EMAIL_HTTP_BIND_ADDR", address.to_string())
            .env("NVBES_EMAIL_GRPC_BIND_ADDR", address.to_string())
            .env("NVBES_EMAIL_FROM_EMAIL", "no-reply@nvbes.fr")
            .env("NVBES_EMAIL_PROVIDER", "test-capture")
            .env("NVBES_EMAIL_TEST_CAPTURE_DIR", &captures)
            .env(
                "NVBES_EMAIL_PRODUCER_TOKENS",
                format!("identity-service={TOKEN}"),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        drop(listener);
        let child = command.spawn().unwrap();
        let mut worker = Self {
            child,
            command,
            captures,
            endpoint: format!("http://{address}"),
        };
        worker.ready().await;
        worker
    }

    async fn ready(&mut self) {
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();
        for _ in 0..100 {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "Email worker exited"
            );
            if http
                .get(format!("{}/health/ready", self.endpoint))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("Email worker did not become ready");
    }

    async fn client(&self, token: &str) -> EmailClient {
        EmailClient::connect(
            EmailClientConfig::from_values(
                "test",
                self.endpoint.clone(),
                token.into(),
                Duration::from_secs(3),
            )
            .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn restart(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
        self.child = self.command.spawn().unwrap();
        self.ready().await;
    }
}

async fn wait_for_capture(db: &PgPool, count: i64) {
    for _ in 0..100 {
        let accepted: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM email_messages WHERE state='provider_accepted'",
        )
        .fetch_one(db)
        .await
        .unwrap();
        if accepted == count {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("provider capture was not accepted");
}

#[tokio::test]
async fn real_mfa_notifications_cross_grpc_and_survive_email_restart_without_duplicates() {
    let identity = crate::test_fixtures::isolated_database().await;
    let schema = format!("email_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&identity)
        .await
        .unwrap();
    let mut url = reqwest::Url::parse(&std::env::var("DATABASE_URL").unwrap()).unwrap();
    url.query_pairs_mut()
        .append_pair("options", &format!("-csearch_path={schema}"));
    let email = PgPool::connect(url.as_str()).await.unwrap();
    sqlx::migrate!("../email-worker/migrations")
        .run(&email)
        .await
        .unwrap();
    let mut worker = Worker::start(url.as_str()).await;
    let client = worker.client(TOKEN).await;

    let (session, token) = crate::test_fixtures::session(&identity).await;
    sqlx::query("INSERT INTO identity_login_identifiers(id,principal_id,kind,normalized_value,verified_at) SELECT $1,principal_id,'email','owner@example.invalid',clock_timestamp() FROM identity_sessions WHERE id=$2")
        .bind(Uuid::new_v4()).bind(session).execute(&identity).await.unwrap();
    let crypto = crate::mfa_crypto::MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
    let pending = crate::totp::start(&identity, &crypto, &token)
        .await
        .unwrap();
    let code = nvbes_core::mfa::generate_totp_code(
        &pending.secret_base32,
        nvbes_core::mfa::current_counter(Utc::now()),
    );
    crate::totp::confirm(&identity, &crypto, &token, pending.factor_id, &code)
        .await
        .unwrap();
    let recovery = crate::mfa_recovery::generate(&identity, &token)
        .await
        .unwrap();
    let saved: serde_json::Value =
        sqlx::query_scalar("SELECT command FROM identity_security_notifications")
            .fetch_one(&identity)
            .await
            .unwrap();
    let command: EmailCommand = serde_json::from_value(saved).unwrap();

    // Email accepts the command, then Identity loses the acknowledgement before settlement.
    let original = client.send(command.clone()).await.unwrap();
    assert!(!original.duplicate);
    wait_for_capture(&email, 1).await;
    worker.restart().await;
    let client = worker.client(TOKEN).await;
    let result = crate::notification_dispatch::run_batch(&identity, &client)
        .await
        .unwrap();
    assert_eq!((result.claimed, result.accepted, result.failed), (1, 1, 0));
    let receipt: String = sqlx::query_scalar("SELECT receipt_id FROM identity_security_notifications WHERE state='accepted' AND command IS NULL AND email_accepted_at IS NOT NULL").fetch_one(&identity).await.unwrap();
    assert_eq!(receipt, original.message_id);
    let replay = client.send(command).await.unwrap();
    assert!(replay.duplicate);
    assert_eq!(replay.accepted_at, original.accepted_at);

    crate::mfa_recovery::redeem(&identity, &token, &recovery.codes[0])
        .await
        .unwrap();
    let bad_client = worker
        .client("wrong-identity-email-runtime-token-32-value")
        .await;
    let denied = crate::notification_dispatch::run_batch(&identity, &bad_client)
        .await
        .unwrap();
    assert_eq!((denied.failed, denied.accepted), (1, 0));
    let failed: String = sqlx::query_scalar("SELECT outcome FROM identity_security_notifications WHERE state='failed' AND command IS NULL").fetch_one(&identity).await.unwrap();
    assert_eq!(failed, "unauthorized");
    let messages: i64 = sqlx::query_scalar("SELECT count(*) FROM email_messages")
        .fetch_one(&email)
        .await
        .unwrap();
    let attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM email_delivery_attempts")
        .fetch_one(&email)
        .await
        .unwrap();
    assert_eq!((messages, attempts), (1, 1));
    let files: Vec<_> = std::fs::read_dir(&worker.captures)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let body = std::fs::read_to_string(&files[0]).unwrap();
    let capture: serde_json::Value = serde_json::from_str(&body).unwrap();
    let provider_id: String = sqlx::query_scalar("SELECT provider_message_id FROM email_messages WHERE id=$1 AND provider_accepted_at IS NOT NULL")
        .bind(Uuid::parse_str(capture["job_id"].as_str().unwrap()).unwrap())
        .fetch_one(&email).await.unwrap();
    assert_eq!(capture["provider_email_id"].as_str().unwrap(), provider_id);
    assert_eq!(capture["to"], serde_json::json!(["owner@example.invalid"]));
    assert!(
        capture["text_body"]
            .as_str()
            .unwrap()
            .contains("recovery codes")
    );
    for secret in recovery
        .codes
        .iter()
        .map(String::as_str)
        .chain([token.as_str(), pending.secret_base32.as_str()])
    {
        assert!(!body.contains(secret));
    }
    drop(worker);
    email.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&identity)
        .await
        .unwrap();
    identity.close().await;
}
