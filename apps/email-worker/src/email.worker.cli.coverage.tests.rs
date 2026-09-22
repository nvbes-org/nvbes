use std::process::{Command, Output};

const DEVELOPMENT_DATA_KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
const DEVELOPMENT_HMAC_KEY: &str = "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";

fn run(args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"));
    command.env_clear().args(args);
    apply_development_env(&mut command);
    command.output().expect("email worker binary runs")
}

fn run_without_config(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"))
        .env_clear()
        .args(args)
        .output()
        .expect("email worker binary runs")
}

fn apply_development_env(command: &mut Command) {
    command
        .env(
            "NVBES_EMAIL_DATABASE_URL",
            "postgres://localhost/nvbes_email_test",
        )
        .env("NVBES_EMAIL_FROM_EMAIL", "no-reply@nvbes.fr")
        .env("NVBES_EMAIL_PROVIDER", "mock");
}

#[test]
fn migrate_requires_a_database_url() {
    let output = run_without_config(&["migrate"]);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_EMAIL_DATABASE_URL"), "{error}");
}

#[test]
fn synthetic_smoke_requires_explicit_recipient_configuration() {
    let output = run_without_config(&["synthetic-smoke"]);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_EMAIL_SYNTHETIC_RECIPIENT"), "{error}");
}

#[test]
fn validate_runtime_accepts_development_defaults() {
    let output = run(&["validate-runtime"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("email runtime configuration is valid"));
}

#[test]
fn error_reporting_smoke_emits_json_with_development_defaults() {
    let output = run(&["error-reporting-smoke"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json smoke");
    assert!(payload.is_object());
}

#[test]
fn unknown_cli_action_reports_usage() {
    let output = run(&["not-a-real-command"]);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("usage: nvbes-email-worker"), "{error}");
}

#[test]
fn production_configuration_requires_explicit_producer_tokens() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"));
    command
        .env_clear()
        .arg("validate-runtime")
        .env("NVBES_ENVIRONMENT", "production")
        .env(
            "NVBES_EMAIL_DATABASE_URL",
            "postgres://localhost/nvbes_email_test",
        )
        .env("NVBES_EMAIL_FROM_EMAIL", "no-reply@nvbes.fr")
        .env("NVBES_EMAIL_PROVIDER", "mock")
        .env("SENTRY_DSN", "https://public@example.invalid/1")
        .env(
            "NVBES_OBSERVABILITY_INTERNAL_TOKEN",
            "01234567890123456789012345678901",
        )
        .env("NVBES_EMAIL_DATA_ENCRYPTION_KEY", DEVELOPMENT_DATA_KEY)
        .env("NVBES_EMAIL_RECIPIENT_HMAC_KEY", DEVELOPMENT_HMAC_KEY);
    let output = command.output().expect("email worker binary runs");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_EMAIL_PRODUCER_TOKENS"), "{error}");
}
