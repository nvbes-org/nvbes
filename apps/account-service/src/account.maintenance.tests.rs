use std::process::{Command, Output};

fn run(action: &str, database_url: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-account-service"));
    command
        .env_clear()
        .arg(action)
        .env("NVBES_ENVIRONMENT", "production")
        .env(
            "NVBES_ACCOUNT_SYNTHETIC_OWNER_ID",
            "00000000-0000-4000-8000-000000000001",
        )
        .env(
            "NVBES_ACCOUNT_SYNTHETIC_MEMBER_ID",
            "00000000-0000-4000-8000-000000000002",
        );
    if let Some(url) = database_url {
        command.env("NVBES_ACCOUNT_DATABASE_URL", url);
    }
    command.output().expect("Account binary runs")
}

#[test]
fn maintenance_requires_an_explicit_database_url() {
    for action in ["migrate", "synthetic-account-smoke", "process-privacy-jobs"] {
        for url in [None, Some("  ")] {
            let output = run(action, url);
            assert!(!output.status.success());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains("required configuration is missing: NVBES_ACCOUNT_DATABASE_URL"),
                "{action}: {error}"
            );
        }
    }
}

#[test]
fn maintenance_reaches_database_validation_without_http_configuration() {
    for action in ["migrate", "synthetic-account-smoke", "process-privacy-jobs"] {
        let output = run(action, Some("invalid-database-url"));
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("relative URL without a base"),
            "{action}: {error}"
        );
    }
}

fn run_development(action: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-account-service"));
    command
        .env_clear()
        .arg(action)
        .env("NVBES_ENVIRONMENT", "development")
        .env("NVBES_ACCOUNT_DATABASE_URL", "postgres://localhost/account")
        .env("NVBES_IDENTITY_TOKEN_KEY_ID", "identity-key-1")
        .env(
            "NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM",
            "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAwEKbtGra6bWscKp5s8i3\njvE0mUrZV54vaDWEfYxnjJYc6TNPBUuHlNUOv44eePZ+TxLQ9wPYSwuhSJIPAX7c\nAkDDdzVJy36lUVTjKGND7PZtgv6ItPQb6yM7YNyM7+QHZXL0fvB5q7O1AasgT+sF\ns0ffePjSyi9QpOww8TqcgePyXN3anUmB8pwoaJQfJOOLE1sJ3zsfw+n3nG+31Lpg\nDQBwYQWrKRRH/R7aYRFoZu1ZRFINfYXVmOGUuehcYkAprNs1dte6szKyyc2zhUJi\nTUez2NsFzgzx1Q1ssxYOMbQcVqNjux5l2aC9i5VcPi7gRHWGKiN77sSjnz90vqSu\nYwIDAQAB\n-----END PUBLIC KEY-----\n",
        );
    command.output().expect("Account binary runs")
}

#[test]
fn unknown_command_prints_usage() {
    let output = run_development("not-a-real-command");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("usage: nvbes-account-service"), "{error}");
}

#[test]
fn synthetic_smoke_rejects_invalid_owner_uuid() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-account-service"));
    command
        .env_clear()
        .arg("synthetic-account-smoke")
        .env("NVBES_ENVIRONMENT", "production")
        .env("NVBES_ACCOUNT_DATABASE_URL", "postgres://localhost/account")
        .env("NVBES_ACCOUNT_SYNTHETIC_OWNER_ID", "not-a-uuid")
        .env(
            "NVBES_ACCOUNT_SYNTHETIC_MEMBER_ID",
            "00000000-0000-4000-8000-000000000002",
        );
    let output = command.output().expect("Account binary runs");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("invalid character"), "{error}");
}

#[test]
fn http_runtime_still_requires_its_configuration() {
    let output = run("validate-runtime", Some("postgres://localhost/account"));
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("NVBES_ACCOUNT_DATABASE_MAX_CONNECTIONS"),
        "{error}"
    );
}
