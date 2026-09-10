use super::*;
use crate::{
    test_fixtures::{isolated_database, session},
    webauthn::{registration, step_up},
};
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};
use webauthn_rs::Webauthn;

#[path = "identity.webauthn.revocation.tests.rs"]
mod revocation;

struct Fixture {
    db: PgPool,
    token: String,
    session: Uuid,
    ids: Vec<Uuid>,
    server: Webauthn,
    authenticator: WebauthnAuthenticator<SoftPasskey>,
}
impl Fixture {
    async fn new(count: usize) -> Self {
        let db = isolated_database().await;
        let (session, token) = session(&db).await;
        let mut f = Self {
            db,
            token,
            session,
            ids: vec![],
            server: crate::webauthn::build_server("identity.example", "https://identity.example")
                .unwrap(),
            authenticator: WebauthnAuthenticator::new(SoftPasskey::new(true)),
        };
        for _ in 0..count {
            f.authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
            let started = registration::start(&f.db, &f.server, &f.token)
                .await
                .unwrap();
            let response = f
                .authenticator
                .do_registration("https://identity.example".parse().unwrap(), started.options)
                .unwrap();
            f.ids.push(
                registration::finish(
                    &f.db,
                    &f.server,
                    &f.token,
                    started.ceremony_id,
                    &response,
                    "Key",
                )
                .await
                .unwrap(),
            );
            let token = f.token.clone();
            f.authenticate(&token).await;
        }
        // Manage from a fresh session that used only the final authenticator.
        let (management, token) = crate::test_fixtures::session(&f.db).await;
        sqlx::query("UPDATE identity_sessions SET principal_id=(SELECT principal_id FROM identity_sessions WHERE id=$1) WHERE id=$2")
            .bind(f.session).bind(management).execute(&f.db).await.unwrap();
        f.authenticate(&token).await;
        f.session = management;
        f.token = token;
        f
    }
    async fn authenticate(&mut self, token: &str) {
        let started = step_up::start(&self.db, &self.server, token).await.unwrap();
        let assertion = self
            .authenticator
            .do_authentication("https://identity.example".parse().unwrap(), started.options)
            .unwrap();
        step_up::finish(
            &self.db,
            &self.server,
            token,
            started.ceremony_id,
            &assertion,
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn owned_metadata_can_be_renamed_and_revoked_once_but_last_factor_is_kept() {
    let f = Fixture::new(2).await;
    let rows = list(&f.db, &f.token).await.unwrap();
    assert_eq!(rows.len(), 2);
    let json = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 4);
    rename(&f.db, &f.token, f.ids[0], "Laptop").await.unwrap();
    rename(&f.db, &f.token, f.ids[0], "Laptop").await.unwrap();
    revoke(&f.db, &f.token, f.ids[0]).await.unwrap();
    revoke(&f.db, &f.token, f.ids[0]).await.unwrap();
    assert_eq!(list(&f.db, &f.token).await.unwrap().len(), 1);
    assert!(matches!(
        revoke(&f.db, &f.token, f.ids[1]).await,
        Err(WebauthnError::LastFactor)
    ));
    for event in ["identity.webauthn.renamed", "identity.webauthn.revoked"] {
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM identity_audit_events WHERE event_type=$1")
                .bind(event)
                .fetch_one(&f.db)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }
}

#[tokio::test]
async fn wrong_owner_stale_authentication_and_invalid_labels_are_rejected() {
    let mut f = Fixture::new(2).await;
    let (_, other) = session(&f.db).await;
    assert!(list(&f.db, &other).await.unwrap().is_empty());
    let enrollment = registration::start(&f.db, &f.server, &other).await.unwrap();
    let response = f
        .authenticator
        .do_registration(
            "https://identity.example".parse().unwrap(),
            enrollment.options,
        )
        .unwrap();
    registration::finish(
        &f.db,
        &f.server,
        &other,
        enrollment.ceremony_id,
        &response,
        "Other account",
    )
    .await
    .unwrap();
    f.authenticate(&other).await;
    assert!(rename(&f.db, &other, f.ids[0], "Intruder").await.is_err());
    assert!(revoke(&f.db, &other, f.ids[0]).await.is_err());
    assert!(
        rename(&f.db, &f.token, f.ids[0], "bad\nlabel")
            .await
            .is_err()
    );
    sqlx::query("UPDATE identity_sessions SET step_up_at=clock_timestamp()-interval '6 minutes' WHERE id=$1").bind(f.session).execute(&f.db).await.unwrap();
    assert!(matches!(
        revoke(&f.db, &f.token, f.ids[0]).await,
        Err(WebauthnError::InvalidSession)
    ));
    assert!(list(&f.db, &f.token).await.is_ok());
}

#[tokio::test]
async fn concurrent_revocations_in_two_sessions_preserve_one_factor() {
    let mut f = Fixture::new(2).await;
    let (id, token) = session(&f.db).await;
    sqlx::query("UPDATE identity_sessions SET principal_id=(SELECT principal_id FROM identity_sessions WHERE id=$1) WHERE id=$2").bind(f.session).bind(id).execute(&f.db).await.unwrap();
    f.authenticate(&token).await;
    let (a, b) = tokio::join!(
        revoke(&f.db, &f.token, f.ids[0]),
        revoke(&f.db, &token, f.ids[1])
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert!(
        matches!(
            a,
            Err(WebauthnError::LastFactor | WebauthnError::InvalidSession)
        ) || matches!(
            b,
            Err(WebauthnError::LastFactor | WebauthnError::InvalidSession)
        )
    );
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_webauthn_credentials WHERE revoked_at IS NULL",
    )
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(active, 1);
}

#[tokio::test]
async fn failed_audit_rolls_back_revocation_and_pending_assertion_survives() {
    let mut f = Fixture::new(2).await;
    let started = step_up::start(&f.db, &f.server, &f.token).await.unwrap();
    let response = f
        .authenticator
        .do_authentication("https://identity.example".parse().unwrap(), started.options)
        .unwrap();
    let credential: Uuid =
        sqlx::query_scalar("SELECT id FROM identity_webauthn_credentials WHERE credential_id=$1")
            .bind(response.get_credential_id())
            .fetch_one(&f.db)
            .await
            .unwrap();
    sqlx::query("ALTER TABLE identity_audit_events ADD CONSTRAINT injected_revoke_failure CHECK (event_type<>'identity.webauthn.revoked')").execute(&f.db).await.unwrap();
    assert!(matches!(
        revoke(&f.db, &f.token, credential).await,
        Err(WebauthnError::Database(_))
    ));
    assert!(
        step_up::finish(&f.db, &f.server, &f.token, started.ceremony_id, &response)
            .await
            .is_ok()
    );
    sqlx::query("ALTER TABLE identity_audit_events DROP CONSTRAINT injected_revoke_failure")
        .execute(&f.db)
        .await
        .unwrap();
    revoke(&f.db, &f.token, credential).await.unwrap();
}

#[tokio::test]
async fn only_an_active_totp_can_replace_the_last_passkey() {
    let f = Fixture::new(1).await;
    let factor = Uuid::new_v4();
    let crypto = crate::mfa_crypto::MfaCrypto::with_rotation(1, [44; 32], None).unwrap();
    let sealed = crypto.seal(factor, "JBSWY3DPEHPK3PXP").unwrap();
    sqlx::query("INSERT INTO identity_auth_factors(id,principal_id,kind,state,secret_ciphertext,secret_nonce,key_version) SELECT $1,principal_id,'totp','pending',$2,$3,$4 FROM identity_sessions WHERE id=$5")
        .bind(factor).bind(sealed.ciphertext).bind(sealed.nonce.as_slice()).bind(sealed.key_version).bind(f.session).execute(&f.db).await.unwrap();
    assert!(matches!(
        revoke(&f.db, &f.token, f.ids[0]).await,
        Err(WebauthnError::LastFactor)
    ));
    sqlx::query("UPDATE identity_auth_factors SET state='active' WHERE id=$1")
        .bind(factor)
        .execute(&f.db)
        .await
        .unwrap();
    revoke(&f.db, &f.token, f.ids[0]).await.unwrap();
    assert!(matches!(
        list(&f.db, &f.token).await,
        Err(WebauthnError::InvalidSession)
    ));
}

#[tokio::test]
async fn http_management_routes_protect_session_and_return_only_owned_metadata() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use tower::ServiceExt;
    let f = Fixture::new(2).await;
    let browser = crate::browser::BrowserSecurity::new("https://identity.example", false).unwrap();
    let browser_token = browser.browser_cookie().token;
    let csrf = browser
        .session_csrf_token(&f.token, &browser_token)
        .unwrap();
    let app = crate::oauth::http::webauthn_router(
        f.db.clone(),
        browser,
        Arc::new(f.server),
        crate::rate_limits::RateLimiter::new(rand::random()).unwrap(),
    );
    for (path, body, origin, status) in [
        (
            "list",
            serde_json::json!({}),
            "https://evil.example",
            StatusCode::FORBIDDEN,
        ),
        (
            "rename",
            serde_json::json!({}),
            "https://evil.example",
            StatusCode::FORBIDDEN,
        ),
        (
            "revoke",
            serde_json::json!({}),
            "https://evil.example",
            StatusCode::FORBIDDEN,
        ),
        (
            "list",
            serde_json::json!({}),
            "https://identity.example",
            StatusCode::OK,
        ),
        (
            "rename",
            serde_json::json!({"credential_id":f.ids[0],"label":"Phone"}),
            "https://identity.example",
            StatusCode::OK,
        ),
        (
            "revoke",
            serde_json::json!({"credential_id":f.ids[0]}),
            "https://identity.example",
            StatusCode::OK,
        ),
        (
            "revoke",
            serde_json::json!({"credential_id":f.ids[1]}),
            "https://identity.example",
            StatusCode::CONFLICT,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/oauth/session/webauthn/credentials/{path}"))
                    .extension(axum::extract::ConnectInfo(
                        "127.0.0.1:9000".parse::<std::net::SocketAddr>().unwrap(),
                    ))
                    .header("origin", origin)
                    .header("content-type", "application/json")
                    .header("x-csrf-token", &csrf)
                    .header(
                        "cookie",
                        format!(
                            "__Host-nvbes-browser={browser_token}; __Host-nvbes-session={}",
                            f.token
                        ),
                    )
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        if path == "list" && status == StatusCode::OK {
            assert_eq!(json.as_array().unwrap().len(), 2);
        }
        if status == StatusCode::CONFLICT {
            assert_eq!(json, serde_json::json!({"error":"last_strong_factor"}));
        }
    }
}
