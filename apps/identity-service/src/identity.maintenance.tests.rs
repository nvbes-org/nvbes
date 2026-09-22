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
