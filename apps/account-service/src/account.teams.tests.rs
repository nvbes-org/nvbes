use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use super::{create_and_join_for_synthetic, hash_join_code, join_team, router};
use crate::{
    profile,
    test_support::{access_token, state_with_pool},
};

#[test]
fn hash_join_code_is_stable_sha256() {
    let first = hash_join_code("team_0123456789abcdef0123456789abcdef");
    let second = hash_join_code("team_0123456789abcdef0123456789abcdef");
    let other = hash_join_code("team_ffffffffffffffffffffffffffffffff");
    assert_eq!(first, second);
    assert_ne!(first, other);
    assert_eq!(first.len(), 32);
}

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

#[sqlx::test(migrations = "./migrations")]
async fn teams_http_create_list_and_join(pool: PgPool) {
    let owner = Uuid::new_v4();
    let member = Uuid::new_v4();
    let state = state_with_pool(pool);
    let app = router(state);
    let owner_token = access_token(owner, "account:write account:read", false);
    let member_token = access_token(member, "account:write account:read", false);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams")
                .header("authorization", format!("Bearer {owner_token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Coverage Team"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(create.into_body(), 64 * 1024)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let join_code = created["join_code"].as_str().unwrap().to_owned();

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/teams")
                .header("authorization", format!("Bearer {owner_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);

    let join = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams/join")
                .header("authorization", format!("Bearer {member_token}"))
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"join_code":"{join_code}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(join.status(), StatusCode::OK);

    let bad_name = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/teams")
                .header("authorization", format!("Bearer {owner_token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_name.status(), StatusCode::BAD_REQUEST);
}
