use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::error::AccountError;

use super::{UpdatePreferences, router, validate_language, validate_theme};
use crate::test_support::{access_token, state_with_pool};

#[test]
fn theme_validation_accepts_supported_values_only() {
    assert!(validate_theme("system").is_ok());
    assert!(validate_theme("light").is_ok());
    assert!(validate_theme("dark").is_ok());
    assert!(matches!(
        validate_theme("neon"),
        Err(AccountError::Invalid("unsupported theme"))
    ));
}

#[test]
fn language_validation_accepts_fr_and_en_only() {
    assert!(validate_language("fr").is_ok());
    assert!(validate_language("en").is_ok());
    assert!(matches!(
        validate_language("de"),
        Err(AccountError::Invalid("unsupported language"))
    ));
}

#[test]
fn update_preferences_struct_holds_validated_fields() {
    let input = UpdatePreferences {
        theme: "dark".into(),
        language: "en".into(),
    };
    assert!(validate_theme(&input.theme).is_ok());
    assert!(validate_language(&input.language).is_ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn preferences_and_notifications_http_round_trip(pool: PgPool) {
    let principal_id = Uuid::new_v4();
    let state = state_with_pool(pool);
    let app = router(state);
    let read = access_token(principal_id, "account:read", false);
    let write = access_token(principal_id, "account:write", false);

    let get_prefs = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/preferences")
                .header("authorization", format!("Bearer {read}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_prefs.status(), StatusCode::OK);

    let put_prefs = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/preferences")
                .header("authorization", format!("Bearer {write}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"theme":"dark","language":"en"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_prefs.status(), StatusCode::OK);

    let bad_theme = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/preferences")
                .header("authorization", format!("Bearer {write}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"theme":"neon","language":"en"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_theme.status(), StatusCode::BAD_REQUEST);

    let get_notifications = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/notifications")
                .header("authorization", format!("Bearer {read}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_notifications.status(), StatusCode::OK);

    let put_notifications = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/notifications")
                .header("authorization", format!("Bearer {write}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":false,"push":true,"in_app":true,"marketing_email":false}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_notifications.status(), StatusCode::OK);
}
