use std::process::{Command, Output};

fn run(args: &[&str], database_url: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-billing-worker"));
    command.env_clear().args(args);
    apply_development_env(&mut command);
    if let Some(url) = database_url {
        command.env("NVBES_BILLING_DATABASE_URL", url);
    }
    command.output().expect("billing worker binary runs")
}

fn apply_development_env(command: &mut Command) {
    command
        .env(
            "NVBES_BILLING_DATABASE_URL",
            std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@127.0.0.1:5432/nvbes_billing".into()
            }),
        )
        .env("NVBES_APP_URL", "https://nvbes.test");
}

#[test]
fn migrate_requires_a_database_url() {
    let output = Command::new(env!("CARGO_BIN_EXE_nvbes-billing-worker"))
        .env_clear()
        .arg("migrate")
        .output()
        .expect("billing worker runs");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_BILLING_DATABASE_URL"), "{error}");
}

#[test]
fn validate_runtime_accepts_development_defaults() {
    let output = run(&["validate-runtime"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("billing worker runtime configuration is valid"));
}

#[test]
fn unknown_cli_action_reports_usage() {
    let output = run(&["not-a-real-command"], None);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("usage: nvbes-billing-worker"), "{error}");
}

#[test]
fn synthetic_smoke_requires_database_connectivity() {
    let output = run(
        &["synthetic-smoke"],
        Some("postgres://127.0.0.1:1/unreachable"),
    );
    assert!(!output.status.success());
}

#[test]
fn migrate_applies_billing_schema_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping migrate integration: postgres unavailable");
        return;
    }
    let output = run(&["migrate"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("billing database migrations applied"));
}

#[test]
fn validate_runtime_rejects_live_stripe_credentials() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-billing-worker"));
    command
        .env_clear()
        .arg("validate-runtime")
        .env(
            "NVBES_BILLING_DATABASE_URL",
            "postgres://postgres:postgres@127.0.0.1:5432/nvbes_billing",
        )
        .env(
            "NVBES_STRIPE_SECRET_KEY",
            format!("{}_{}", "sk_live", "forbidden_in_v1"),
        );
    let output = command.output().expect("billing worker runs");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("Live Stripe credentials"), "{error}");
}

#[test]
fn synthetic_smoke_runs_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping synthetic-smoke integration: postgres unavailable");
        return;
    }
    let output = run(&["synthetic-smoke"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json smoke");
    assert!(payload["event_dispatched"].as_bool().unwrap_or(false));
}

#[test]
fn deployment_bootstrap_serves_live_health_check() {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let addr = "127.0.0.1:18082";
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvbes-billing-worker"))
        .env_clear()
        .arg("deployment-bootstrap")
        .env("NVBES_BILLING_HTTP_BIND_ADDR", addr)
        .spawn()
        .expect("spawn bootstrap");

    let response = (0..20).find_map(|_| {
        std::thread::sleep(Duration::from_millis(50));
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

fn postgres_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
}
