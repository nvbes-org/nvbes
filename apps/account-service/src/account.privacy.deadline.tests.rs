use super::{claims, config, sign};
use axum::{
    Json, Router,
    body::Body,
    http::{Request, StatusCode},
    routing::post,
};
use serde_json::json;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone, Copy)]
enum Action {
    Export,
    OutboxExport,
    Closure,
    Cancel,
    Download,
    ExistingExport,
    ExistingClosure,
    FirstExport,
    FirstClosure,
}

async fn check(action: Action, expire: bool) {
    let pool = crate::database::connect(&std::env::var("DATABASE_URL").unwrap(), 4)
        .await
        .unwrap();
    crate::database::migrate(&pool).await.unwrap();
    let owner = Uuid::new_v4();
    let resource = Uuid::new_v4();
    if !matches!(action, Action::FirstExport | Action::FirstClosure) {
        crate::profile::ensure_profile(&pool, owner).await.unwrap();
    }
    if matches!(action, Action::Cancel | Action::ExistingClosure) {
        sqlx::query("INSERT INTO account_closures(id,principal_id,status,execute_after) VALUES($1,$2,'pending',clock_timestamp()+interval '7 days')").bind(resource).bind(owner).execute(&pool).await.unwrap();
        sqlx::query(
            "UPDATE account_profiles SET lifecycle_status='closure_pending' WHERE principal_id=$1",
        )
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    }
    if matches!(action, Action::ExistingExport) {
        sqlx::query("INSERT INTO account_exports(id,principal_id,status) VALUES($1,$2,'pending')")
            .bind(resource)
            .bind(owner)
            .execute(&pool)
            .await
            .unwrap();
    }
    if matches!(action, Action::Download) {
        sqlx::query("INSERT INTO account_exports(id,principal_id,status,document,completed_at,expires_at) VALUES($1,$2,'completed','{}',clock_timestamp(),clock_timestamp()+interval '1 hour')").bind(resource).bind(owner).execute(&pool).await.unwrap();
    }
    let before = snapshot(&pool, owner).await;
    // Generate test keys before starting the short deadline.
    let mut cfg = config();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    cfg.token_issuer = format!("http://{}", listener.local_addr().unwrap());
    let mut claims = claims();
    let until = chrono::Utc::now().timestamp() + if expire { 3 } else { 60 };
    claims["iss"] = json!(cfg.token_issuer);
    claims["sub"] = json!(owner);
    claims["step_up_expires_at"] = json!(until);
    let token = sign(&claims);
    claims["active"] = json!(true);
    claims["token_type"] = json!("Bearer");
    let identity = Router::new().route(
        "/oauth/introspect",
        post(move || {
            let claims = claims.clone();
            async move { Json(claims) }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, identity).await.unwrap();
    });
    let state = crate::app::AccountState::new(cfg, pool.clone()).unwrap();
    let app = crate::privacy::router(state);
    let mut blocker = pool.begin().await.unwrap();
    let table = match action {
        Action::OutboxExport => "account_outbox",
        Action::Download | Action::ExistingExport => "account_exports",
        Action::ExistingClosure => "account_closures",
        _ => "account_audit_events",
    };
    sqlx::query(&format!("LOCK TABLE {table} IN ACCESS EXCLUSIVE MODE"))
        .execute(&mut *blocker)
        .await
        .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let (method, path) = match action {
        Action::Export | Action::OutboxExport | Action::ExistingExport | Action::FirstExport => {
            ("POST", "/api/v1/privacy/exports".into())
        }
        Action::Closure | Action::FirstClosure | Action::ExistingClosure => {
            ("POST", "/api/v1/closure".into())
        }
        Action::Cancel => ("POST", "/api/v1/closure/cancel".into()),
        Action::Download => (
            "GET",
            format!("/api/v1/privacy/exports/{resource}/document"),
        ),
    };
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let pending = tokio::spawn(async move { app.oneshot(request).await.unwrap() });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
            assert!(
                !pending.is_finished(),
                "request must reach the actual SQL lock"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("actual blocked waiter");
    if expire {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let expired: bool = sqlx::query_scalar(
                    "SELECT clock_timestamp()>=to_timestamp($1::double precision)",
                )
                .bind(until as f64)
                .fetch_one(&pool)
                .await
                .unwrap();
                if expired {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("proof deadline elapsed in PostgreSQL");
    }
    blocker.rollback().await.unwrap();
    let response = tokio::time::timeout(Duration::from_secs(5), pending)
        .await
        .unwrap()
        .unwrap();
    server.abort();
    if expire {
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            snapshot(&pool, owner).await,
            before,
            "rollback must include profile, preferences, audit and outbox"
        );
    } else {
        assert!(
            response.status().is_success(),
            "fresh proof rejected: {}",
            response.status()
        );
    }
    pool.close().await;
}

async fn snapshot(pool: &PgPool, owner: Uuid) -> serde_json::Value {
    sqlx::query_scalar("SELECT jsonb_build_object('profile',(SELECT to_jsonb(p) FROM account_profiles p WHERE principal_id=$1),'preferences',(SELECT to_jsonb(p) FROM account_preferences p WHERE principal_id=$1),'exports',(SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM account_exports e WHERE principal_id=$1),'closures',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM account_closures c WHERE principal_id=$1),'audit',(SELECT count(*) FROM account_audit_events WHERE principal_id=$1),'outbox',(SELECT count(*) FROM account_outbox WHERE payload->>'principal_id'=$1::text))").bind(owner).fetch_one(pool).await.unwrap()
}

macro_rules! deadline_test {
    ($name:ident, $action:ident) => {
        #[tokio::test]
        async fn $name() {
            check(Action::$action, true).await;
        }
    };
}
deadline_test!(export_rejects_expired_proof_after_audit_wait, Export);
deadline_test!(export_rejects_expired_proof_after_outbox_wait, OutboxExport);
deadline_test!(
    first_closure_rolls_back_profile_after_expired_proof,
    FirstClosure
);
deadline_test!(closure_rejects_expired_proof_after_audit_wait, Closure);
deadline_test!(cancel_rejects_expired_proof_after_audit_wait, Cancel);
deadline_test!(download_rejects_expired_proof_after_read_wait, Download);
deadline_test!(
    existing_export_rejects_expired_proof_after_read_wait,
    ExistingExport
);
deadline_test!(
    existing_closure_rejects_expired_proof_after_read_wait,
    ExistingClosure
);
deadline_test!(
    first_export_rolls_back_profile_after_expired_proof,
    FirstExport
);

#[tokio::test]
async fn fresh_proof_allows_each_privacy_action_after_a_lock_wait() {
    for action in [
        Action::Export,
        Action::OutboxExport,
        Action::Closure,
        Action::Cancel,
        Action::Download,
        Action::ExistingExport,
        Action::ExistingClosure,
        Action::FirstExport,
        Action::FirstClosure,
    ] {
        check(action, false).await;
    }
}
