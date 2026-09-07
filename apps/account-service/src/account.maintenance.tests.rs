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
