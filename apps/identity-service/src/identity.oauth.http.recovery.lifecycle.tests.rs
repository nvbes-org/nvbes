use super::*;
const RESUME: &str = "/oauth/recovery/resume";
const CANCEL: &str = "/oauth/recovery/cancel";

#[tokio::test]
async fn resume_preserves_expiry_and_cancel_retires_the_ceremony_without_restoring_authority() {
    let f = Fixture::new().await;
    let codes = f.codes().await;
    let (cookie, csrf) = f.redeem(&codes[0], &f.cookie(), &f.csrf).await;
    let expiry: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT expires_at FROM identity_mfa_recovery_sessions WHERE principal_id=$1",
    )
    .bind(f.principal)
    .fetch_one(&f.db)
    .await
    .unwrap();
    let nonce = crate::oauth::store::random_secret();
    let (status, headers, state) = f.send(RESUME, "{}", &cookie, &nonce, ORIGIN).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state["csrf_token"], csrf);
    assert_eq!(
        serde_json::from_value::<chrono::DateTime<chrono::Utc>>(state["expires_at"].clone())
            .unwrap(),
        expiry
    );
    assert!(state.get("token").is_none());
    assert!(!headers.contains_key(header::SET_COOKIE));
    assert_eq!(
        f.send(CANCEL, "{}", &cookie, &nonce, ORIGIN).await.0,
        StatusCode::FORBIDDEN
    );
    let (_, _, started) = f.send(OPTIONS, "{}", &cookie, &csrf, ORIGIN).await;
    let credential = WebauthnAuthenticator::new(SoftPasskey::new(true))
        .do_registration(
            ORIGIN.parse().unwrap(),
            serde_json::from_value(started["options"].clone()).unwrap(),
        )
        .unwrap();
    let (status, headers, body) = f.send(CANCEL, "{}", &cookie, &csrf, ORIGIN).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        serde_json::json!({"cancelled":true,"must_reauthenticate":true})
    );
    let cleared = headers[header::SET_COOKIE].to_str().unwrap();
    assert!(cleared.starts_with("__Host-nvbes-recovery=") && cleared.contains("Max-Age=0"));
    for path in [RESUME, CANCEL, OPTIONS] {
        assert_eq!(
            f.send(path, "{}", &cookie, &csrf, ORIGIN).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let finish = serde_json::json!({"ceremony_id":started["ceremony_id"],"credential":credential,"label":"Too late"}).to_string();
    assert_eq!(
        f.send(FINISH, &finish, &cookie, &csrf, ORIGIN).await.0,
        StatusCode::BAD_REQUEST
    );
    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM identity_sessions WHERE principal_id=$1 AND revoked_at IS NULL),(SELECT count(*) FROM identity_auth_factors WHERE principal_id=$1 AND state='active'),(SELECT count(*) FROM identity_mfa_recovery_codes WHERE principal_id=$1 AND consumed_at IS NOT NULL),(SELECT count(*) FROM identity_webauthn_challenges WHERE recovery_session_id IS NOT NULL),(SELECT count(*) FROM identity_audit_events WHERE principal_id=$1 AND event_type='identity.mfa_recovery_cancelled')")
        .bind(f.principal).fetch_one(&f.db).await.unwrap();
    assert_eq!(counts, (0, 1, 1, 0, 1));
}

#[tokio::test]
async fn bootstrap_and_cancel_fail_closed_and_count_ambiguous_requests() {
    let f = Fixture::new().await;
    let codes = f.codes().await;
    let (cookie, csrf) = f.redeem(&codes[0], &f.cookie(), &f.csrf).await;
    for path in [RESUME, CANCEL] {
        assert_eq!(
            f.send(path, "{", &cookie, &csrf, "https://evil.example")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            f.send(path, "{}", &cookie, "", ORIGIN).await.0,
            StatusCode::FORBIDDEN
        );
        let duplicate = format!("{cookie}; {}", cookie.split("; ").nth(1).unwrap());
        assert!(matches!(
            f.send(path, "{}", &duplicate, &csrf, ORIGIN).await.0,
            StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
        ));
        for body in ["[]", "null", "{\"extra\":true}"] {
            assert_eq!(
                f.send(path, body, &cookie, &csrf, ORIGIN).await.0,
                StatusCode::BAD_REQUEST
            );
        }
    }
    for _ in 0..14 {
        f.limiter
            .check(&f.db, Category::WebauthnAccount, &f.principal.to_string())
            .await
            .unwrap();
    }
    for path in [RESUME, CANCEL] {
        let (status, headers, _) = f.send(path, "{}", &cookie, &csrf, ORIGIN).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert!(!headers.contains_key(header::SET_COOKIE));
    }
}

#[tokio::test]
async fn cancellation_failure_preserves_recovery_and_expired_or_suspended_recovery_cannot_resume() {
    let f = Fixture::new().await;
    let codes = f.codes().await;
    let (cookie, csrf) = f.redeem(&codes[0], &f.cookie(), &f.csrf).await;
    sqlx::query("ALTER TABLE identity_outbox ADD CONSTRAINT cancel_failure CHECK(event_type<>'identity.mfa_recovery_cancelled')").execute(&f.db).await.unwrap();
    let (status, headers, _) = f.send(CANCEL, "{}", &cookie, &csrf, ORIGIN).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!headers.contains_key(header::SET_COOKIE));
    assert_eq!(
        f.send(RESUME, "{}", &cookie, &csrf, ORIGIN).await.0,
        StatusCode::OK
    );
    sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=$1")
        .bind(f.principal)
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.send(RESUME, "{}", &cookie, &csrf, ORIGIN).await.0,
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE identity_principals SET status='active' WHERE id=$1")
        .bind(f.principal)
        .execute(&f.db)
        .await
        .unwrap();
    sqlx::query("UPDATE identity_mfa_recovery_sessions SET created_at=clock_timestamp()-interval '10 minutes',expires_at=clock_timestamp()-interval '1 second' WHERE principal_id=$1").bind(f.principal).execute(&f.db).await.unwrap();
    for path in [RESUME, CANCEL] {
        assert_eq!(
            f.send(path, "{}", &cookie, &csrf, ORIGIN).await.0,
            StatusCode::BAD_REQUEST
        );
    }
}
