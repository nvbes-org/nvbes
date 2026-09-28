#![allow(unused_imports)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use super::{create_and_join_for_synthetic, hash_join_code, join_team, router};
use crate::profile;
use crate::test_support::{access_token, state_with_pool};

#[test]
fn hash_join_code_is_stable_sha256() {
    let first = hash_join_code("team_0123456789abcdef0123456789abcdef");
    let second = hash_join_code("team_0123456789abcdef0123456789abcdef");
    let other = hash_join_code("team_ffffffffffffffffffffffffffffffff");
    assert_eq!(first, second);
    assert_ne!(first, other);
    assert_eq!(first.len(), 32);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn create_and_join_synthetic_assigns_member_role(pool: PgPool) {
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    let (team_id, role) = create_and_join_for_synthetic(&pool, owner, member)
        .await
        .unwrap();
    assert_eq!(role, "member");
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_team_memberships WHERE team_id=$1")
            .bind(team_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn join_team_rejects_malformed_codes(pool: PgPool) {
    let principal = Uuid::new_v4();
    profile::ensure_profile(&pool, principal).await.unwrap();
    assert!(matches!(
        join_team(&pool, principal, "bad", Uuid::new_v4()).await,
        Err(crate::error::AccountError::NotFound)
    ));
    assert!(matches!(
        join_team(&pool, principal, "team_short", Uuid::new_v4()).await,
        Err(crate::error::AccountError::NotFound)
    ));
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn teams_http_create_list_and_join(pool: PgPool) {
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    profile::ensure_profile(&pool, owner).await.unwrap();
    profile::ensure_profile(&pool, member).await.unwrap();

    let state = state_with_pool(pool);
    let app = router(state);
    let owner_write = access_token(owner, "account:write", false);
    let owner_read = access_token(owner, "account:read", false);
    let member_write = access_token(member, "account:write", false);

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams")
                .header("authorization", format!("Bearer {owner_write}"))
                .header("content-type", "application/json")
                .header("x-correlation-id", Uuid::new_v4().to_string())
                .body(Body::from(r#"{"name":"Coverage team"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = axum::body::to_bytes(created.into_body(), usize::MAX)
        .await
        .unwrap();
    let created_json: serde_json::Value = serde_json::from_slice(&created_body).unwrap();
    let join_code = created_json["join_code"].as_str().expect("join code");

    let invalid_name = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams")
                .header("authorization", format!("Bearer {owner_write}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_name.status(), StatusCode::BAD_REQUEST);

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/teams")
                .header("authorization", format!("Bearer {owner_read}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);

    let joined = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams/join")
                .header("authorization", format!("Bearer {member_write}"))
                .header("content-type", "application/json")
                .header("x-correlation-id", Uuid::new_v4().to_string())
                .body(Body::from(format!(r#"{{"join_code":"{join_code}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(joined.status(), StatusCode::OK);
}
