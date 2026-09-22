use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use chrono::Utc;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use super::{ClosureStatus, ExportStatus, router, with_closure_participant, with_participant};
use crate::{
    profile,
    test_support::{access_token, state_with_pool},
};

#[test]
fn export_and_closure_participants_mirror_row_status() {
    let export = with_participant(ExportStatus {
        export_id: Uuid::nil(),
        status: "completed".into(),
        requested_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: Some(Utc::now()),
        expires_at: Some(Utc::now()),
        last_error: None,
        participants: Vec::new(),
    });
    assert_eq!(export.participants.len(), 1);
    assert_eq!(export.participants[0].participant, "account");
    assert_eq!(export.participants[0].status, "completed");
    assert_eq!(export.participants[0].attempts, 1);
    assert!(export.participants[0].completed_at.is_some());

    let closure = with_closure_participant(ClosureStatus {
        saga_id: Uuid::nil(),
        status: "pending".into(),
        requested_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: None,
        last_error: Some("late".into()),
        participants: Vec::new(),
    });
    assert_eq!(closure.participants[0].status, "pending");
    assert_eq!(closure.participants[0].last_error.as_deref(), Some("late"));
}

#[sqlx::test(migrations = "./migrations")]
async fn export_and_closure_http_lifecycle(pool: PgPool) {
    let principal_id = Uuid::new_v4();
    let state = state_with_pool(pool);
    let app = router(state);
    let token = access_token(
        principal_id,
        "account:export account:close account:read account:write",
        true,
    );

    let export = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/privacy/exports")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export.status(), StatusCode::ACCEPTED);

    let export_again = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/privacy/exports")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export_again.status(), StatusCode::ACCEPTED);

    let latest = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/privacy/exports/latest")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(latest.status(), StatusCode::OK);

    let closure = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/closure")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(closure.status(), StatusCode::ACCEPTED);

    let get_closure = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/closure")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_closure.status(), StatusCode::OK);

    let cancel = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/closure/cancel")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cancel.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "./migrations")]
async fn download_export_requires_completed_document(pool: PgPool) {
    let principal_id = Uuid::new_v4();
    let export_id = Uuid::new_v4();
    profile::ensure_profile(&pool, principal_id).await.unwrap();
    sqlx::query("INSERT INTO account_exports(id,principal_id,status) VALUES($1,$2,'pending')")
        .bind(export_id)
        .bind(principal_id)
        .execute(&pool)
        .await
        .unwrap();

    let state = state_with_pool(pool);
    let app = router(state);
    let token = access_token(principal_id, "account:export", true);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/privacy/exports/{export_id}/document"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let _ = to_bytes(response.into_body(), 1024).await;
}

#[tokio::test]
async fn privacy_routes_reject_unauthenticated_calls() {
    let database_url = "postgres://account:account@127.0.0.1:1/account_test";
    let db = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(10))
        .connect_lazy(database_url)
        .unwrap();
    let state = crate::app::AccountState::new(crate::test_support::account_config(), db).unwrap();
    let response = router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/privacy/exports")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
