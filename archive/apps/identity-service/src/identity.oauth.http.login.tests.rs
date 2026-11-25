use super::*;

const PASSWORD: &str = "Atomic-hosted-login-password!";

async fn credentials(f: &Fixture) -> (uuid::Uuid, String) {
    let email = format!("atomic-login-{}@example.invalid", uuid::Uuid::new_v4());
    let principal = crate::auth::create_synthetic_identity(&f.db, &email, PASSWORD)
        .await
        .unwrap();
    (principal, email)
}

fn login_body(authorization: &serde_json::Value, email: &str) -> String {
    serde_json::json!({"interaction": authorization["interaction"], "email": email, "password": PASSWORD}).to_string()
}

async fn counts(f: &Fixture, principal: uuid::Uuid) -> (i64, i64) {
    sqlx::query_as("SELECT (SELECT count(*) FROM identity_sessions WHERE principal_id=$1),(SELECT count(*) FROM identity_audit_events WHERE principal_id=$1 AND event_type='identity.authenticated')")
        .bind(principal).fetch_one(&f.db).await.unwrap()
}

#[tokio::test]
async fn invalid_interaction_csrf_cannot_leave_an_authenticated_session() {
    let f = Fixture::new().await;
    let (principal, email) = credentials(&f).await;
    let (_, authorization) = f.authorize().await;
    let body = login_body(&authorization, &email);
    let response = f
        .post(
            "/oauth/authorize/login",
            &random_secret(),
            &body,
            "https://identity.example",
        )
        .await;
    assert_eq!(response.0, StatusCode::BAD_REQUEST);
    assert!(!response.1.contains_key("set-cookie"));
    assert_eq!(counts(&f, principal).await, (0, 0));
    assert_eq!(
        f.post(
            "/oauth/authorize/login",
            authorization["csrf_token"].as_str().unwrap(),
            &body,
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(counts(&f, principal).await, (1, 1));
}

#[tokio::test]
async fn concurrent_login_submissions_create_one_session_and_one_audit() {
    let f = Fixture::new().await;
    let (principal, email) = credentials(&f).await;
    let (_, authorization) = f.authorize().await;
    let csrf = authorization["csrf_token"].as_str().unwrap();
    let body = login_body(&authorization, &email);
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        tokio::join!(
            f.post(
                "/oauth/authorize/login",
                csrf,
                &body,
                "https://identity.example"
            ),
            f.post(
                "/oauth/authorize/login",
                csrf,
                &body,
                "https://identity.example"
            )
        )
    })
    .await
    .expect("concurrent login submissions must finish");
    assert_ne!(a.0 == StatusCode::OK, b.0 == StatusCode::OK);
    let loser = if a.0 == StatusCode::OK { b } else { a };
    assert_eq!(loser.0, StatusCode::BAD_REQUEST);
    assert!(!loser.1.contains_key("set-cookie"));
    assert_eq!(counts(&f, principal).await, (1, 1));
}

#[tokio::test]
async fn failed_interaction_update_rolls_back_session_audit_and_csrf_rotation() {
    let f = Fixture::new().await;
    let (principal, email) = credentials(&f).await;
    let (_, authorization) = f.authorize().await;
    let csrf = authorization["csrf_token"].as_str().unwrap();
    let body = login_body(&authorization, &email);
    // Fault injection is confined to this test's private PostgreSQL schema.
    sqlx::query("CREATE FUNCTION fail_login_binding() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected binding failure'; END $$")
        .execute(&f.db).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_login_binding BEFORE UPDATE ON identity_oauth_requests FOR EACH ROW EXECUTE FUNCTION fail_login_binding()")
        .execute(&f.db).await.unwrap();
    let response = f
        .post(
            "/oauth/authorize/login",
            csrf,
            &body,
            "https://identity.example",
        )
        .await;
    assert_eq!(response.0, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!response.1.contains_key("set-cookie"));
    assert_eq!(counts(&f, principal).await, (0, 0));
    sqlx::query("DROP TRIGGER fail_login_binding ON identity_oauth_requests")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.post(
            "/oauth/authorize/login",
            csrf,
            &body,
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(counts(&f, principal).await, (1, 1));
}

#[tokio::test]
async fn credentials_changed_after_verification_cannot_create_a_session() {
    for mutation in [
        "UPDATE identity_principals SET status='suspended' WHERE id=$1",
        "UPDATE identity_password_credentials SET password_hash='changed-after-verification' WHERE principal_id=$1",
        "UPDATE identity_login_identifiers SET normalized_value='changed@example.invalid' WHERE principal_id=$1",
    ] {
        let f = Fixture::new().await;
        let (principal, email) = credentials(&f).await;
        let verified = crate::auth::verify_credentials(&f.db, &email, PASSWORD)
            .await
            .unwrap();
        sqlx::query(mutation)
            .bind(principal)
            .execute(&f.db)
            .await
            .unwrap();
        let mut tx = f.db.begin().await.unwrap();
        assert!(
            crate::auth::create_verified_session(&mut tx, verified)
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
        assert_eq!(counts(&f, principal).await, (0, 0));
    }
}

#[tokio::test]
async fn swapping_the_displayed_session_cannot_authenticate_another_account() {
    let mut f = Fixture::new().await;
    let (principal, email) = credentials(&f).await;
    let (_, authorization) = f.authorize().await;
    let (_, other) = session(&f.db).await;
    f.token = other;
    let response = f
        .post(
            "/oauth/authorize/login",
            authorization["csrf_token"].as_str().unwrap(),
            &login_body(&authorization, &email),
            "https://identity.example",
        )
        .await;
    assert_eq!(response.0, StatusCode::BAD_REQUEST);
    assert_eq!(counts(&f, principal).await, (0, 0));
}
