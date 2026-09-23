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

#[test]
fn deployment_bootstrap_serves_live_health_check() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral bind");
    let addr = listener.local_addr().expect("local addr").to_string();
    drop(listener);

    let mut child = Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"))
        .env_clear()
        .arg("deployment-bootstrap")
        .env("NVBES_EMAIL_HTTP_BIND_ADDR", &addr)
        .spawn()
        .expect("spawn bootstrap");

    let response = (0..40).find_map(|_| {
        std::thread::sleep(Duration::from_millis(50));
        if let Some(status) = child.try_wait().ok().flatten() {
            panic!("bootstrap exited early: {status}");
        }
        TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(100))
            .and_then(|mut stream| {
                stream.write_all(b"GET /health/live HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
                let mut buf = [0_u8; 128];
                let read = stream.read(&mut buf)?;
                Ok(String::from_utf8_lossy(&buf[..read]).to_string())
            })
            .ok()
    });

    let _ = child.kill();
    let _ = child.wait();
    let body = response.expect("health response");
    assert!(body.contains("204") || body.contains("200"), "{body}");
}

#[test]
fn deployment_bootstrap_rejects_invalid_bind_address() {
    let output = Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"))
        .env_clear()
        .arg("deployment-bootstrap")
        .env("NVBES_EMAIL_HTTP_BIND_ADDR", "not-a-socket-addr")
        .output()
        .expect("email worker binary runs");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_EMAIL_HTTP_BIND_ADDR"), "{error}");
}

#[test]
fn serve_starts_http_health_on_ephemeral_port() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral bind");
    let addr = listener.local_addr().expect("local addr").to_string();
    drop(listener);

    let mut child = Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"))
        .env_clear()
        .env(
            "NVBES_EMAIL_DATABASE_URL",
            "postgres://localhost/nvbes_email_test",
        )
        .env("NVBES_EMAIL_FROM_EMAIL", "no-reply@nvbes.fr")
        .env("NVBES_EMAIL_PROVIDER", "mock")
        .env("NVBES_EMAIL_HTTP_BIND_ADDR", &addr)
        .env("NVBES_EMAIL_GRPC_BIND_ADDR", &addr)
        .spawn()
        .expect("spawn serve");

    let response = (0..60).find_map(|_| {
        std::thread::sleep(Duration::from_millis(50));
        if let Some(status) = child.try_wait().ok().flatten() {
            panic!("serve exited early: {status}");
        }
        TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(100))
            .and_then(|mut stream| {
                stream.write_all(b"GET /health/live HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
                let mut buf = [0_u8; 256];
                let read = stream.read(&mut buf)?;
                Ok(String::from_utf8_lossy(&buf[..read]).to_string())
            })
            .ok()
    });

    let _ = child.kill();
    let _ = child.wait();
    let body = response.expect("health response");
    assert!(
        body.contains("200") || body.contains("alive") || body.contains("nvbes-email-worker"),
        "{body}"
    );
}

#[test]
fn migrate_fails_against_unreachable_database() {
    let output = Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"))
        .env_clear()
        .arg("migrate")
        .env(
            "NVBES_EMAIL_DATABASE_URL",
            "postgres://127.0.0.1:1/unreachable",
        )
        .output()
        .expect("email worker binary runs");
    assert!(!output.status.success());
}

#[test]
fn release_suppression_requires_four_arguments() {
    let output = run(&["release-suppression", "not-enough"]);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("usage: nvbes-email-worker"), "{error}");
}

#[test]
fn synthetic_smoke_requires_run_id_when_recipient_is_set() {
    let output = Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"))
        .env_clear()
        .arg("synthetic-smoke")
        .env("NVBES_EMAIL_SYNTHETIC_RECIPIENT", "operator@example.com")
        .output()
        .expect("email worker binary runs");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_EMAIL_SYNTHETIC_RUN_ID"), "{error}");
}

#[test]
fn migrate_applies_email_schema_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping migrate integration: postgres unavailable");
        return;
    }
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let db_name = format!("nvbes_email_migrate_{suffix}");
    let admin_url = "postgres://postgres:postgres@127.0.0.1:5432/postgres";
    let create = Command::new("psql")
        .args([
            admin_url,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("CREATE DATABASE {db_name}"),
        ])
        .output()
        .expect("create database");
    assert!(
        create.status.success(),
        "{}",
        String::from_utf8_lossy(&create.stderr)
    );
    let database_url = format!("postgres://postgres:postgres@127.0.0.1:5432/{db_name}");
    let output = Command::new(env!("CARGO_BIN_EXE_nvbes-email-worker"))
        .env_clear()
        .arg("migrate")
        .env("NVBES_EMAIL_DATABASE_URL", &database_url)
        .output()
        .expect("email worker binary runs");
    let drop = Command::new("psql")
        .args([
            admin_url,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("DROP DATABASE IF EXISTS {db_name}"),
        ])
        .output()
        .expect("drop database");
    assert!(
        drop.status.success(),
        "{}",
        String::from_utf8_lossy(&drop.stderr)
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn postgres_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
}
