use std::process::{Command, Output};

fn run(args: &[&str], database_url: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-identity-service"));
    command.env_clear().args(args);
    apply_development_env(&mut command);
    if let Some(url) = database_url {
        command.env("NVBES_IDENTITY_DATABASE_URL", url);
    }
    command.output().expect("identity binary runs")
}

fn apply_development_env(command: &mut Command) {
    command
        .env("NVBES_ENVIRONMENT", "development")
        .env(
            "NVBES_IDENTITY_DATABASE_URL",
            std::env::var("DATABASE_URL")
                .or_else(|_| std::env::var("NVBES_SECURITY_TEST_DATABASE_URL"))
                .unwrap_or_else(|_| {
                    "postgres://postgres:postgres@127.0.0.1:15432/nvbes_coverage_test".into()
                }),
        )
        .env(
            "NVBES_IDENTITY_MFA_ENCRYPTION_KEY",
            "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=",
        )
        .env("NVBES_IDENTITY_MFA_KEY_VERSION", "1");
}

fn postgres_reachable() -> bool {
    let candidate = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_SECURITY_TEST_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://127.0.0.1:15432/postgres".into());
    let Some(without_scheme) = candidate.split("://").nth(1) else {
        return false;
    };
    let Some(authority) = without_scheme.split('/').next() else {
        return false;
    };
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let addr = match host_port.rsplit_once(':') {
        Some((host, port)) => format!("{host}:{port}"),
        None => format!("{host_port}:5432"),
    };
    let Ok(addr) = addr.parse() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(200)).is_ok()
}

fn spawn_on_ephemeral(
    args: &[&str],
    bind_env: &str,
    extra_env: &[(&str, String)],
) -> (std::process::Child, String) {
    use std::io::Read;
    use std::net::TcpListener;
    use std::time::Duration;

    let mut last_err = String::from("no attempts");
    for _ in 0..8 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral bind");
        let addr = listener.local_addr().expect("local addr").to_string();
        drop(listener);

        let mut command = Command::new(env!("CARGO_BIN_EXE_nvbes-identity-service"));
        command
            .env_clear()
            .args(args)
            .env(bind_env, &addr)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        for (key, value) in extra_env {
            command.env(key, value);
        }
        apply_development_env(&mut command);
        let mut child = command.spawn().expect("spawn identity");
        std::thread::sleep(Duration::from_millis(80));
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                last_err = format!("exited {status}: {stderr}");
            }
            Ok(None) => return (child, addr),
            Err(err) => {
                last_err = err.to_string();
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    panic!("could not bind ephemeral health listener after retries: {last_err}");
}

fn await_live_health(child: &mut std::process::Child, addr: &str) -> String {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let response = (0..100).find_map(|_| {
        std::thread::sleep(Duration::from_millis(100));
        if let Some(status) = child.try_wait().ok().flatten() {
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr);
            }
            panic!("process exited early: {status}; stderr={stderr}");
        }
        TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(200))
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
    response.expect("health response")
}

#[test]
fn migrate_requires_an_explicit_database_url() {
    let output = Command::new(env!("CARGO_BIN_EXE_nvbes-identity-service"))
        .env_clear()
        .arg("migrate")
        .env("NVBES_ENVIRONMENT", "production")
        .output()
        .expect("identity binary runs");
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_IDENTITY_DATABASE_URL"), "{error}");
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
    assert!(stdout.contains("identity runtime configuration is valid"));
}

#[test]
fn unknown_cli_action_reports_usage() {
    let output = run(&["not-a-real-command"], None);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("usage: nvbes-identity-service"), "{error}");
}

#[test]
fn migrate_applies_identity_schema_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping migrate integration: postgres unavailable");
        return;
    }

    let admin = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
    });
    let db_name = format!(
        "identity_cli_{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let disposable_url = match admin.rfind('/') {
        Some(idx) => format!("{}{db_name}", &admin[..=idx]),
        None => {
            eprintln!("skipping migrate integration: invalid DATABASE_URL");
            return;
        }
    };

    let create = Command::new("psql")
        .args([
            &admin,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("CREATE DATABASE {db_name}"),
        ])
        .output()
        .expect("create disposable database");
    if !create.status.success() {
        eprintln!(
            "skipping migrate integration: {}",
            String::from_utf8_lossy(&create.stderr)
        );
        return;
    }

    let output = run(&["migrate"], Some(&disposable_url));
    let _ = Command::new("psql")
        .args([
            &admin,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("DROP DATABASE IF EXISTS {db_name}"),
        ])
        .output();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("identity database migrations applied"));
}

#[test]
fn bootstrap_serves_live_health_check_then_stops() {
    let (mut child, addr) = spawn_on_ephemeral(&[], "NVBES_IDENTITY_BIND_ADDR", &[]);
    let body = await_live_health(&mut child, &addr);
    assert!(body.contains("200") || body.contains("alive"), "{body}");
}

#[test]
fn synthetic_auth_smoke_requires_explicit_credentials() {
    let output = run(&["synthetic-auth-smoke"], None);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("NVBES_IDENTITY_SYNTHETIC_EMAIL"), "{error}");
}

#[test]
fn help_style_unknown_action_lists_supported_commands() {
    let output = run(&["--help"], None);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("migrate"), "{error}");
    assert!(error.contains("validate-runtime"), "{error}");
}
