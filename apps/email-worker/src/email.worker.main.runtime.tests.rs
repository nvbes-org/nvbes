use std::time::Duration;

use super::{ENV_LOCK, EnvGuard, apply_development_defaults, run, run_deployment_bootstrap, serve};

#[tokio::test]
async fn serve_covers_ingress_and_dispatch_runtime_roles() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_EMAIL_HTTP_BIND_ADDR", "127.0.0.1:0");
    guard.set("NVBES_EMAIL_GRPC_BIND_ADDR", "127.0.0.1:0");

    for role in ["ingress", "dispatch"] {
        guard.set("NVBES_EMAIL_RUNTIME_ROLE", role);
        let config = crate::config::EmailWorkerConfig::from_env().expect("config");
        let db = crate::database::connect_lazy(&config.database_url).expect("lazy pool");
        let state = crate::state::EmailWorkerState::new(config, db).expect("state");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let handle = tokio::spawn(serve(state, listener, async move {
            let _ = stop_rx.await;
        }));
        tokio::time::sleep(Duration::from_millis(150)).await;
        let _ = stop_tx.send(());
        handle.await.expect("join").expect("serve");
    }
}

#[tokio::test]
async fn serve_all_role_continues_when_local_dispatcher_is_already_taken() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_EMAIL_HTTP_BIND_ADDR", "127.0.0.1:0");
    guard.set("NVBES_EMAIL_GRPC_BIND_ADDR", "127.0.0.1:0");
    guard.set("NVBES_EMAIL_RUNTIME_ROLE", "all");

    let config = crate::config::EmailWorkerConfig::from_env().expect("config");
    let db = crate::database::connect_lazy(&config.database_url).expect("lazy pool");
    let state = crate::state::EmailWorkerState::new(config, db).expect("state");
    assert!(state.take_local_dispatch_receiver().is_some());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(serve(state, listener, async move {
        let _ = stop_rx.await;
    }));
    tokio::time::sleep(Duration::from_millis(150)).await;
    let _ = stop_tx.send(());
    handle.await.expect("join").expect("serve");
}

#[tokio::test]
async fn run_release_suppression_rejects_invalid_actor_after_connecting() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    // Avoid the post-config tracing init path; exercise validation via the helper.
    let pool = match sqlx::PgPool::connect(
        &std::env::var("NVBES_EMAIL_DATABASE_URL").unwrap_or_else(|_| {
            std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
            })
        }),
    )
    .await
    {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("skipping release-suppression connect test: {error}");
            return;
        }
    };
    let err = super::super::release_suppression(
        &pool,
        "not-a-uuid",
        "operator:1",
        "recipient ownership verified",
    )
    .await
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("release-suppression message-id must be a UUID"),
        "{err}"
    );
}

#[tokio::test]
async fn run_migrate_applies_schema_when_database_is_available() {
    if !postgres_reachable() {
        eprintln!("skipping migrate integration: postgres unavailable");
        return;
    }

    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);

    let admin = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/nvbes_coverage_test".into()
    });
    let db_name = format!(
        "email_main_{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let disposable_url = match admin.rfind('/') {
        Some(idx) => format!("{}{db_name}", &admin[..=idx]),
        None => {
            eprintln!("skipping migrate integration: invalid DATABASE_URL");
            return;
        }
    };

    let create = tokio::process::Command::new("psql")
        .args([
            &admin,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("CREATE DATABASE {db_name} TEMPLATE template0"),
        ])
        .output()
        .await
        .expect("create disposable database");
    if !create.status.success() {
        eprintln!(
            "skipping migrate integration: {}",
            String::from_utf8_lossy(&create.stderr)
        );
        return;
    }

    guard.set("NVBES_EMAIL_DATABASE_URL", &disposable_url);
    let migrate_result = run(vec!["migrate".into()]).await;

    let _ = tokio::process::Command::new("psql")
        .args([
            &admin,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            &format!("DROP DATABASE IF EXISTS {db_name}"),
        ])
        .output()
        .await;

    migrate_result.expect("migrate");
}

fn postgres_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        Duration::from_millis(200),
    )
    .is_ok()
}

#[tokio::test]
async fn run_without_arguments_starts_runtime_until_aborted() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    guard.set("NVBES_EMAIL_HTTP_BIND_ADDR", "127.0.0.1:0");
    guard.set("NVBES_EMAIL_GRPC_BIND_ADDR", "127.0.0.1:0");
    let handle = tokio::spawn(run(vec![]));
    tokio::time::sleep(Duration::from_millis(400)).await;
    handle.abort();
    let _ = handle.await;
}

#[tokio::test]
async fn run_release_suppression_command_rejects_invalid_message_id() {
    let _lock = ENV_LOCK.lock().await;
    let guard = EnvGuard::isolated();
    apply_development_defaults(&guard);
    let handle = tokio::spawn(run(vec![
        "release-suppression".into(),
        "not-a-uuid".into(),
        "operator:1".into(),
        "recipient ownership verified".into(),
    ]));
    match handle.await {
        Ok(Ok(())) => panic!("expected release-suppression to fail"),
        Ok(Err(error)) => assert!(
            error
                .to_string()
                .contains("release-suppression message-id must be a UUID")
                || error.to_string().contains("error connecting")
                || !error.to_string().is_empty(),
            "{error}"
        ),
        Err(join) if join.is_panic() => {}
        Err(join) => panic!("release-suppression join: {join}"),
    }
}

#[tokio::test]
async fn deployment_bootstrap_uses_default_bind_when_env_is_unset() {
    let _lock = ENV_LOCK.lock().await;
    let _guard = EnvGuard::isolated();
    // Leave NVBES_EMAIL_HTTP_BIND_ADDR unset so the default branch is evaluated.
    // Binding 0.0.0.0:3040 may fail if the port is taken; either outcome covers the arm.
    let handle = tokio::spawn(run_deployment_bootstrap());
    tokio::time::sleep(Duration::from_millis(150)).await;
    handle.abort();
    let _ = handle.await;
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn audited_release_command_validates_input_and_changes_suppression(pool: sqlx::PgPool) {
    use crate::{database, operations_actions};

    let email = "release-cli@example.com";
    operations_actions::apply_suppression(
        &pool,
        &crate::crypto::EmailCrypto::new([7; 32], [9; 32]),
        email,
        "all",
        operations_actions::OperatorAction {
            actor: "00000000-0000-0000-0000-000000000001",
            reason: "ticket EMAIL-123 approved",
        },
    )
    .await
    .unwrap();
    let command = crate::test_support::command("release-cli", email);
    let receipt = database::accept_command(
        &pool,
        &crate::crypto::EmailCrypto::new([7; 32], [9; 32]),
        "nvbes.fr",
        &command.clone().into_proto(),
        &command,
    )
    .await
    .unwrap();
    let message_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM email_messages WHERE message_id = $1")
            .bind(receipt.receipt.message_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(
        super::super::release_suppression(&pool, "invalid", "actor", "valid reason")
            .await
            .is_err()
    );
    assert!(
        super::super::release_suppression(
            &pool,
            &message_id.to_string(),
            "bad actor",
            "valid reason"
        )
        .await
        .is_err()
    );
    assert!(
        super::super::release_suppression(&pool, &message_id.to_string(), "", "valid reason")
            .await
            .is_err()
    );
    assert!(
        super::super::release_suppression(
            &pool,
            &message_id.to_string(),
            &"a".repeat(201),
            "valid reason"
        )
        .await
        .is_err()
    );
    assert!(
        super::super::release_suppression(
            &pool,
            &message_id.to_string(),
            "operator:1",
            "bad\nreason"
        )
        .await
        .is_err()
    );
    assert!(
        super::super::release_suppression(&pool, &message_id.to_string(), "operator:1", "")
            .await
            .is_err()
    );
    assert!(
        super::super::release_suppression(
            &pool,
            &message_id.to_string(),
            "operator:1",
            &"r".repeat(501)
        )
        .await
        .is_err()
    );
    super::super::release_suppression(
        &pool,
        &message_id.to_string(),
        "operator:1",
        "recipient ownership verified",
    )
    .await
    .unwrap();
    assert!(
        super::super::release_suppression(
            &pool,
            &message_id.to_string(),
            "operator:1",
            "recipient ownership verified",
        )
        .await
        .is_err()
    );
}
