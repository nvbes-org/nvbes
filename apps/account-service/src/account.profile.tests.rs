#![allow(unused_imports)]
use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
};
use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use super::{ProfileRow, UpdateProfile, clean, ensure_profile, router, validate_profile};
use crate::test_support::{access_token, state_with_pool};

#[test]
fn validate_profile_rejects_oversized_and_future_values() {
    assert!(
        validate_profile(&UpdateProfile {
            firstname: Some("a".repeat(101)),
            lastname: None,
            username: None,
            birthdate: None,
            region: None,
        })
        .is_err()
    );
    assert!(
        validate_profile(&UpdateProfile {
            firstname: None,
            lastname: None,
            username: Some("ab".into()),
            birthdate: None,
            region: None,
        })
        .is_err()
    );
    assert!(
        validate_profile(&UpdateProfile {
            firstname: None,
            lastname: None,
            username: None,
            birthdate: None,
            region: Some("x".into()),
        })
        .is_err()
    );
    assert!(
        validate_profile(&UpdateProfile {
            firstname: None,
            lastname: None,
            username: None,
            birthdate: Some(Utc::now().date_naive() + chrono::Duration::days(1)),
            region: None,
        })
        .is_err()
    );
    assert!(
        validate_profile(&UpdateProfile {
            firstname: Some("Ada".into()),
            lastname: Some("Lovelace".into()),
            username: Some("ada".into()),
            birthdate: Some(NaiveDate::from_ymd_opt(1815, 12, 10).unwrap()),
            region: Some("eu-west".into()),
        })
        .is_ok()
    );
}

#[test]
fn clean_trims_and_drops_empty_values() {
    assert_eq!(clean(Some("  Ada  ".into())).as_deref(), Some("Ada"));
    assert_eq!(clean(Some("   ".into())), None);
    assert_eq!(clean(None), None);
}

#[test]
fn profile_display_name_falls_back_to_username_then_user() {
    let named = ProfileRow {
        principal_id: Uuid::nil(),
        firstname: Some("Ada".into()),
        lastname: Some("Lovelace".into()),
        username: Some("ada".into()),
        birthdate: None,
        region: None,
        created_at: Utc::now(),
    };
    let profile: super::Profile = named.into();
    assert_eq!(profile.display_name, "Ada Lovelace");

    let username_only = ProfileRow {
        principal_id: Uuid::nil(),
        firstname: None,
        lastname: Some("   ".into()),
        username: Some("ada".into()),
        birthdate: None,
        region: None,
        created_at: Utc::now(),
    };
    let profile: super::Profile = username_only.into();
    assert_eq!(profile.display_name, "ada");

    let anonymous = ProfileRow {
        principal_id: Uuid::nil(),
        firstname: None,
        lastname: None,
        username: None,
        birthdate: None,
        region: None,
        created_at: Utc::now(),
    };
    let profile: super::Profile = anonymous.into();
    assert_eq!(profile.display_name, "User");
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn ensure_profile_is_idempotent(pool: PgPool) {
    let principal_id = Uuid::new_v4();
    let first = ensure_profile(&pool, principal_id).await.unwrap();
    let second = ensure_profile(&pool, principal_id).await.unwrap();
    assert_eq!(first.principal_id, principal_id);
    assert_eq!(second.principal_id, principal_id);
}

#[cfg(feature = "database-tests")]
#[sqlx::test(migrations = "./migrations")]
async fn profile_http_get_and_update(pool: PgPool) {
    let principal_id = Uuid::new_v4();
    let state = state_with_pool(pool);
    let app = router(state);
    let read_token = access_token(principal_id, "account:read", false);
    let write_token = access_token(principal_id, "account:write", false);

    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/profile")
                .header("authorization", format!("Bearer {read_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);

    let update = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/profile")
                .header("authorization", format!("Bearer {write_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"firstname":"Ada","lastname":"Lovelace","username":"ada","region":"eu"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
}

#[allow(
    dead_code,
    reason = "type-only HeaderMap import anchor for profile test helpers"
)]
fn _headers(_: HeaderMap) {}
