use crate::error::AccountError;

use super::{UpdatePreferences, validate_language, validate_theme};

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

#[cfg(feature = "database-tests")]
mod database {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sqlx::PgPool;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::test_support::{access_token, state_with_pool};

    use super::super::router;

    #[sqlx::test(migrations = "./migrations")]
    async fn preferences_and_notifications_http_round_trip(pool: PgPool) {
        let principal_id = Uuid::new_v4();
        let state = state_with_pool(pool);
        let app = router(state);
        let read_token = access_token(principal_id, "account:read", false);
        let write_token = access_token(principal_id, "account:write", false);

        let preferences = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/preferences")
                    .header("authorization", format!("Bearer {read_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preferences.status(), StatusCode::OK);

        let updated = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/preferences")
                    .header("authorization", format!("Bearer {write_token}"))
                    .header("content-type", "application/json")
                    .header("x-correlation-id", Uuid::new_v4().to_string())
                    .body(Body::from(r#"{"theme":"dark","language":"en"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(updated.status(), StatusCode::OK);

        let invalid_theme = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/preferences")
                    .header("authorization", format!("Bearer {write_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"theme":"neon","language":"en"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid_theme.status(), StatusCode::BAD_REQUEST);

        let notifications = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/notifications")
                    .header("authorization", format!("Bearer {read_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(notifications.status(), StatusCode::OK);

        let updated_notifications = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/notifications")
                    .header("authorization", format!("Bearer {write_token}"))
                    .header("content-type", "application/json")
                    .header("x-correlation-id", Uuid::new_v4().to_string())
                    .body(Body::from(
                        r#"{"email":false,"push":true,"in_app":true,"marketing_email":false}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(updated_notifications.status(), StatusCode::OK);
    }
}
