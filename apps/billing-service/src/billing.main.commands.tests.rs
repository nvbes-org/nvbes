use super::{
    ENV_LOCK, EnvGuard, apply_development_defaults, postgres_reachable, run,
    run_deployment_bootstrap,
};

#[tokio::test]
async fn run_deployment_bootstrap_via_cli_action() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = probe.local_addr().expect("local addr").to_string();
    drop(probe);
    guard.set("NVBES_BILLING_BIND_ADDR", &addr);

    let handle = tokio::spawn(run(vec!["deployment-bootstrap".into()]));
    let mut response = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if handle.is_finished() {
            break;
        }
        if let Ok(mut stream) = tokio::net::TcpStream::connect(&addr).await {
            if stream
                .write_all(b"GET /health/live HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .await
                .is_err()
            {
                continue;
            }
            let mut buf = [0_u8; 128];
            if let Ok(read) = stream.read(&mut buf).await
                && read > 0
            {
                response = Some(String::from_utf8_lossy(&buf[..read]).to_string());
                break;
            }
        }
    }

    handle.abort();
    let _ = handle.await;
    let body = response.expect("health response");
    assert!(body.contains("204") || body.contains("200"), "{body}");
}

#[tokio::test]
async fn deployment_bootstrap_accepts_legacy_http_bind_addr_env() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral bind");
    let addr = probe.local_addr().expect("local addr").to_string();
    drop(probe);
    guard.set("NVBES_BILLING_HTTP_BIND_ADDR", &addr);

    let handle = tokio::spawn(run_deployment_bootstrap());
    let mut response = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if handle.is_finished() {
            break;
        }
        if let Ok(mut stream) = tokio::net::TcpStream::connect(&addr).await {
            if stream
                .write_all(b"GET /health/live HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .await
                .is_err()
            {
                continue;
            }
            let mut buf = [0_u8; 128];
            if let Ok(read) = stream.read(&mut buf).await
                && read > 0
            {
                response = Some(String::from_utf8_lossy(&buf[..read]).to_string());
                break;
            }
        }
    }

    handle.abort();
    let _ = handle.await;
    let body = response.expect("health response");
    assert!(body.contains("204") || body.contains("200"), "{body}");
}

#[tokio::test]
async fn run_check_stripe_mappings_emits_report_on_isolated_database() {
    if !postgres_reachable() {
        eprintln!("skipping check-stripe-mappings: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);

    // Use a disposable database: shared coverage DB accumulates duplicate
    // stripe_price_id rows from other suites, and non-empty failures call exit(1).
    let admin_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
    });
    let db_name = format!("billing_map_{}", uuid::Uuid::new_v4().simple());
    let isolated_url = match admin_url.rfind('/') {
        Some(idx) => format!("{}{db_name}", &admin_url[..=idx]),
        None => format!("postgres://postgres:postgres@127.0.0.1:5432/{db_name}"),
    };

    let admin = sqlx::PgPool::connect(&admin_url)
        .await
        .expect("admin connect");
    sqlx::query(&format!("CREATE DATABASE \"{db_name}\""))
        .execute(&admin)
        .await
        .expect("create isolated database");
    guard.set("NVBES_BILLING_DATABASE_URL", &isolated_url);

    let result = async {
        run(vec!["migrate".into()]).await?;
        run(vec!["check-stripe-mappings".into()]).await
    }
    .await;

    let _ = sqlx::query(&format!(
        "DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE)"
    ))
    .execute(&admin)
    .await;
    admin.close().await;

    result.expect("check-stripe-mappings");
}

#[tokio::test]
async fn run_check_stripe_mappings_fails_for_unreachable_database() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_BILLING_DATABASE_URL",
        "postgres://127.0.0.1:1/unreachable",
    );
    let err = run(vec!["check-stripe-mappings".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn run_synthetic_billing_smoke_fails_for_unreachable_database() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_BILLING_DATABASE_URL",
        "postgres://127.0.0.1:1/unreachable",
    );
    let err = run(vec!["synthetic-billing-smoke".into()])
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn run_synthetic_billing_smoke_succeeds_with_explicit_ids() {
    if !postgres_reachable() {
        eprintln!("skipping synthetic-billing-smoke: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let workspace_id = uuid::Uuid::new_v4();
    let owner_id = uuid::Uuid::new_v4();
    guard.set(
        "NVBES_BILLING_SYNTHETIC_WORKSPACE_ID",
        workspace_id.to_string(),
    );
    guard.set("NVBES_BILLING_SYNTHETIC_OWNER_ID", owner_id.to_string());
    run(vec!["synthetic-billing-smoke".into()])
        .await
        .expect("synthetic-billing-smoke");
}

#[tokio::test]
async fn run_synthetic_billing_smoke_generates_ids_when_env_missing() {
    if !postgres_reachable() {
        eprintln!("skipping synthetic-billing-smoke defaults: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    run(vec!["synthetic-billing-smoke".into()])
        .await
        .expect("synthetic-billing-smoke");
}

#[tokio::test]
async fn run_publish_outbox_reports_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping publish-outbox: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    run(vec!["publish-outbox".into()])
        .await
        .expect("publish-outbox");
}

#[tokio::test]
async fn run_publish_outbox_rejects_invalid_email_client_config() {
    if !postgres_reachable() {
        eprintln!("skipping publish-outbox email config: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_EMAIL_GRPC_ENDPOINT", "http://127.0.0.1:1");
    guard.set("NVBES_BILLING_EMAIL_TOKEN", "short");
    let err = run(vec!["publish-outbox".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty(), "{err}");
}

#[tokio::test]
async fn run_publish_outbox_fails_when_email_endpoint_is_unreachable() {
    if !postgres_reachable() {
        eprintln!("skipping publish-outbox email connect: postgres unavailable");
        return;
    }
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_EMAIL_GRPC_ENDPOINT", "http://127.0.0.1:1");
    guard.set(
        "NVBES_BILLING_EMAIL_TOKEN",
        "billing-email-token-with-enough-length",
    );
    let err = run(vec!["publish-outbox".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty(), "{err}");
}

#[tokio::test]
async fn run_publish_outbox_fails_for_unreachable_database() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set(
        "NVBES_BILLING_DATABASE_URL",
        "postgres://127.0.0.1:1/unreachable",
    );
    let err = run(vec!["publish-outbox".into()]).await.unwrap_err();
    assert!(!err.to_string().is_empty());
}
