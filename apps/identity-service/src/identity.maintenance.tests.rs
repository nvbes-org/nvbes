use std::process::{Command, Output};

fn run(action: &str, database_url: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-identity-service"));
    command
        .env_clear()
        .arg(action)
        .env("NVBES_ENVIRONMENT", "production");
    if let Some(url) = database_url {
        command.env("NVBES_IDENTITY_DATABASE_URL", url);
    }
    command.output().expect("Identity binary runs")
}

fn run_development(action: &str, database_url: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvbes-identity-service"))
        .env_clear()
        .arg(action)
        .env("NVBES_ENVIRONMENT", "development")
        .env("NVBES_IDENTITY_DATABASE_URL", database_url)
        .env(
            "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
            "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=",
        )
        .env("NVBES_IDENTITY_MFA_KEY_VERSION", "1")
        .output()
        .expect("Identity binary runs")
}

#[test]
fn maintenance_requires_an_explicit_database_url() {
    for action in ["migrate", "synthetic-auth-smoke"] {
        for url in [None, Some("  ")] {
            let output = run(action, url);
            assert!(!output.status.success());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains("NVBES_IDENTITY_DATABASE_URL"),
                "{action}: {error}"
            );
        }
    }
}

#[test]
fn unknown_command_prints_usage() {
    let output = run_development("not-a-real-command", "postgres://localhost/identity");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("usage: nvbes-identity-service"), "{error}");
}

#[test]
fn validate_runtime_rejects_invalid_database_url() {
    let output = run_development("validate-runtime", "invalid-database-url");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("relative URL without a base"), "{error}");
}

#[test]
fn validate_runtime_accepts_development_defaults() {
    let output = run_development(
        "validate-runtime",
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_identity",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("identity runtime configuration is valid"));
}

#[test]
fn synthetic_token_smoke_requires_explicit_audience() {
    let output = run_development(
        "synthetic-token-smoke",
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_identity",
    );
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("NVBES_IDENTITY_SYNTHETIC_TOKEN_AUDIENCE")
            || error.contains("NVBES_IDENTITY_SYNTHETIC_EMAIL"),
        "{error}"
    );
}

#[test]
fn rotate_mfa_key_requires_an_explicit_database_url() {
    let output = run("rotate-mfa-key", None);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("NVBES_IDENTITY_DATABASE_URL") || error.contains("database"),
        "{error}"
    );
}

#[test]
fn rotate_mfa_key_reports_zero_when_database_has_no_pending_factors() {
    if !postgres_reachable() {
        eprintln!("skipping rotate-mfa-key integration: postgres unavailable");
        return;
    }
    let output = run_development(
        "rotate-mfa-key",
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_identity",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rotated_factors"));
}

#[test]
fn synthetic_auth_smoke_requires_explicit_credentials() {
    let output = run_development(
        "synthetic-auth-smoke",
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_identity",
    );
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_IDENTITY_SYNTHETIC_EMAIL"), "{error}");
}

#[test]
fn migrate_applies_identity_schema_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping migrate integration: postgres unavailable");
        return;
    }
    let output = run_development(
        "migrate",
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_identity",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("identity database migrations applied"));
}

fn postgres_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
}

#[test]
fn error_reporting_smoke_emits_json_in_development() {
    let output = Command::new(env!("CARGO_BIN_EXE_nvbes-identity-service"))
        .env_clear()
        .arg("error-reporting-smoke")
        .env("NVBES_ENVIRONMENT", "development")
        .env("SENTRY_DSN", "https://public@o.ingest.sentry.io/1")
        .env("SENTRY_TRACES_SAMPLE_RATE", "0")
        .output()
        .expect("Identity binary runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json smoke");
    assert!(payload.is_object());
}
