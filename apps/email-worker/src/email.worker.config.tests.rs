use std::{ffi::OsString, path::PathBuf, sync::Mutex};

use super::{
    DEVELOPMENT_DATA_KEY, DEVELOPMENT_HMAC_KEY, EmailWorkerConfig, database_url_from_env,
    development_value, key, local_smtp, producers,
};

static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

const VARIABLES: &[&str] = &[
    "NVBES_ENVIRONMENT",
    "NVBES_EMAIL_DATABASE_URL",
    "NVBES_EMAIL_HTTP_BIND_ADDR",
    "NVBES_EMAIL_PRODUCER_TOKENS",
    "NVBES_EMAIL_DATA_ENCRYPTION_KEY",
    "NVBES_EMAIL_RECIPIENT_HMAC_KEY",
    "NVBES_EMAIL_FROM_EMAIL",
    "NVBES_EMAIL_FROM_NAME",
    "NVBES_EMAIL_REPLY_TO",
    "NVBES_EMAIL_MESSAGE_ID_DOMAIN",
    "NVBES_EMAIL_PROVIDER",
    "NVBES_EMAIL_TEST_CAPTURE_DIR",
    "NVBES_SCALEWAY_EMAIL_SECRET_KEY",
    "NVBES_SCALEWAY_EMAIL_PROJECT_ID",
    "NVBES_SCALEWAY_EMAIL_REGION",
    "NVBES_SMTP_HOST",
    "NVBES_SMTP_PORT",
    "NVBES_SMTP_USERNAME",
    "NVBES_SMTP_PASSWORD",
    "NVBES_SMTP_STARTTLS",
    "NVBES_EMAIL_SNS_TOPIC_ARN",
    "NVBES_EMAIL_SNS_CA_BUNDLE_PATH",
    "NVBES_EMAIL_SNS_CA_BUNDLE_PEM",
    "NVBES_EMAIL_SNS_CERTIFICATE_HOST",
    "NVBES_EMAIL_SNS_CONFIRMATION_HOST",
    "NVBES_EMAIL_PAYLOAD_RETENTION_DAYS",
    "NVBES_EMAIL_LEDGER_RETENTION_DAYS",
    "NVBES_EMAIL_QUEUE_URL",
    "NVBES_EMAIL_QUEUE_ACCESS_KEY",
    "NVBES_EMAIL_QUEUE_SECRET_KEY",
    "NVBES_EMAIL_QUEUE_REGION",
    "NVBES_EMAIL_QUEUE_ENDPOINT",
    "NVBES_EMAIL_RUNTIME_ROLE",
    "PORT",
];

struct EnvironmentGuard {
    previous: Vec<(&'static str, Option<OsString>)>,
}

impl EnvironmentGuard {
    fn isolated() -> Self {
        let previous = VARIABLES
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in VARIABLES {
            unsafe { std::env::remove_var(name) };
        }
        Self { previous }
    }

    fn set(&self, name: &str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe { std::env::set_var(name, value) };
    }

    fn remove(&self, name: &str) {
        unsafe { std::env::remove_var(name) };
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        for (name, value) in &self.previous {
            match value {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

#[test]
fn cryptographic_keys_must_decode_to_exactly_32_bytes() {
    assert!(key("TEST_KEY", Some(DEVELOPMENT_DATA_KEY)).is_ok());
    assert!(key("TEST_KEY", Some(DEVELOPMENT_HMAC_KEY)).is_ok());
    assert!(key("TEST_KEY", Some("c2hvcnQ=")).is_err());
}

#[test]
fn migration_configuration_requires_only_a_database_url() {
    let _lock = ENVIRONMENT_LOCK.lock().unwrap();
    let environment = EnvironmentGuard::isolated();
    assert!(database_url_from_env().is_err());
    environment.set(
        "NVBES_EMAIL_DATABASE_URL",
        " postgres://localhost/nvbes_email_test ",
    );
    assert_eq!(
        database_url_from_env().unwrap(),
        "postgres://localhost/nvbes_email_test"
    );
}

#[test]
fn producer_credentials_are_bound_and_unique_in_production() {
    assert!(producers::parse(" , ", "production").is_err());
    assert!(
        producers::parse(
            "identity-service=01234567890123456789012345678901,billing-worker=01234567890123456789012345678901",
            "production",
        )
        .is_err()
    );
    assert_eq!(
        producers::parse(
            "identity-service=01234567890123456789012345678901,billing-worker=abcdefghijklmnopqrstuvwxyzABCDEF",
            "production",
        )
        .unwrap()
        .len(),
        2
    );
}

#[test]
fn smtp_is_local_only_and_supports_an_unauthenticated_mail_sink() {
    assert!(
        local_smtp(
            "development",
            "127.0.0.1".to_string(),
            1025,
            None,
            None,
            false,
        )
        .is_ok()
    );
    assert!(
        local_smtp(
            "production",
            "127.0.0.1".to_string(),
            1025,
            None,
            None,
            false,
        )
        .is_err()
    );
    assert!(local_smtp("test", " ".into(), 1025, None, None, false).is_err());
    assert!(
        local_smtp(
            "test",
            "localhost".into(),
            1025,
            Some("user".into()),
            None,
            false,
        )
        .is_err()
    );
}

#[test]
fn producer_parser_rejects_malformed_credentials() {
    assert!(producers::parse("missing-separator", "test").is_err());
    assert!(producers::parse("invalid producer=01234567890123456789012345678901", "test").is_err());
    assert!(producers::parse("identity-service=short", "test").is_err());
    assert!(
        producers::parse(
            "identity-service=01234567890123456789012345678901,identity-service=abcdefghijklmnopqrstuvwxyzABCDEF",
            "test",
        )
        .is_err()
    );
    assert_eq!(producers::from_environment(None, "test").unwrap().len(), 5);
    assert!(producers::from_environment(None, "production").is_err());
}

#[test]
fn environment_configuration_covers_supported_providers_and_guardrails() {
    let _lock = ENVIRONMENT_LOCK.lock().unwrap();
    let environment = EnvironmentGuard::isolated();
    environment.set(
        "NVBES_EMAIL_DATABASE_URL",
        "postgres://localhost/nvbes_email_test",
    );
    environment.set("NVBES_EMAIL_FROM_EMAIL", "no-reply@nvbes.fr");
    environment.set("NVBES_EMAIL_PROVIDER", "mock");

    let development = EmailWorkerConfig::from_env().unwrap();
    assert_eq!(development.environment, "development");
    assert_eq!(development.message_id_domain, "nvbes.fr");
    assert_eq!(development.provider.label(), "mock");
    assert!(development.webhook.is_none());
    assert_eq!(development.retention.payload_days, 30);

    environment.set("NVBES_EMAIL_HTTP_BIND_ADDR", "not-an-address");
    assert!(EmailWorkerConfig::from_env().is_err());
    environment.remove("NVBES_EMAIL_HTTP_BIND_ADDR");
    environment.set("NVBES_EMAIL_FROM_EMAIL", "invalid");
    assert!(EmailWorkerConfig::from_env().is_err());
    environment.set("NVBES_EMAIL_FROM_EMAIL", "no-reply@nvbes.fr");
    environment.set("NVBES_EMAIL_REPLY_TO", "invalid");
    assert!(EmailWorkerConfig::from_env().is_err());
    environment.remove("NVBES_EMAIL_REPLY_TO");

    environment.set("NVBES_EMAIL_PROVIDER", "test-capture");
    environment.set("NVBES_EMAIL_TEST_CAPTURE_DIR", "relative");
    assert!(EmailWorkerConfig::from_env().is_err());
    let capture = std::env::temp_dir().join("nvbes-email-worker-config-capture");
    environment.set("NVBES_EMAIL_TEST_CAPTURE_DIR", &capture);
    assert_eq!(
        EmailWorkerConfig::from_env().unwrap().provider.label(),
        "test-capture"
    );

    environment.set("NVBES_EMAIL_PROVIDER", "smtp");
    environment.set("NVBES_SMTP_PORT", "invalid");
    assert!(EmailWorkerConfig::from_env().is_err());
    environment.set("NVBES_SMTP_PORT", "1025");
    environment.set("NVBES_SMTP_STARTTLS", "invalid");
    assert!(EmailWorkerConfig::from_env().is_err());
    environment.set("NVBES_SMTP_STARTTLS", "true");
    assert_eq!(
        EmailWorkerConfig::from_env().unwrap().provider.label(),
        "smtp"
    );

    environment.set("NVBES_ENVIRONMENT", "production");
    environment.set(
        "NVBES_EMAIL_PRODUCER_TOKENS",
        "identity-service=01234567890123456789012345678901,backoffice-service=abcdefghijklmnopqrstuvwxyzABCDEF",
    );
    environment.set("NVBES_EMAIL_DATA_ENCRYPTION_KEY", DEVELOPMENT_DATA_KEY);
    environment.set("NVBES_EMAIL_RECIPIENT_HMAC_KEY", DEVELOPMENT_HMAC_KEY);
    environment.set("NVBES_EMAIL_PROVIDER", "mock");
    assert!(EmailWorkerConfig::from_env().is_err());
    environment.set("NVBES_EMAIL_PROVIDER", "test-capture");
    assert!(EmailWorkerConfig::from_env().is_err());
    environment.set("NVBES_EMAIL_PROVIDER", "unknown");
    assert!(EmailWorkerConfig::from_env().is_err());

    environment.set("NVBES_EMAIL_PROVIDER", "scaleway");
    environment.set("NVBES_EMAIL_RUNTIME_ROLE", "ingress");
    environment.set("NVBES_SCALEWAY_EMAIL_SECRET_KEY", "secret");
    environment.set("NVBES_SCALEWAY_EMAIL_PROJECT_ID", "project");
    assert!(EmailWorkerConfig::from_env().is_err());
    environment.set("NVBES_EMAIL_QUEUE_URL", "https://sqs.example.test/queue");
    environment.set("NVBES_EMAIL_QUEUE_ACCESS_KEY", "access");
    environment.set("NVBES_EMAIL_QUEUE_SECRET_KEY", "secret");
    environment.set("NVBES_EMAIL_SNS_TOPIC_ARN", "arn:scw:sns:fr-par:test:topic");
    assert!(EmailWorkerConfig::from_env().is_err());
    let ca_path: PathBuf = std::env::temp_dir().join(format!(
        "nvbes-email-worker-ca-{}.pem",
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&ca_path, b"test-ca").unwrap();
    environment.set("NVBES_EMAIL_SNS_CA_BUNDLE_PATH", &ca_path);
    let production = EmailWorkerConfig::from_env().unwrap();
    assert_eq!(production.provider.label(), "scaleway");
    assert_eq!(production.webhook.unwrap().ca_bundle_pem, b"test-ca");
    std::fs::remove_file(ca_path).unwrap();

    assert!(development_value("production", DEVELOPMENT_DATA_KEY).is_none());
}
