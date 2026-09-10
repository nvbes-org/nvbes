use super::*;
use crate::{
    test_fixtures::{self, isolated_database, session},
    webauthn::registration,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

struct Fixture {
    db: PgPool,
    server: Webauthn,
    clients: ClientRegistry,
    handle: String,
    proof: BrowserProof,
    principal: Uuid,
    credential: Uuid,
    authenticator: WebauthnAuthenticator<SoftPasskey>,
}

impl Fixture {
    async fn new() -> Self {
        let db = isolated_database().await;
        let (session, token) = session(&db).await;
        let principal =
            sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
                .bind(session)
                .fetch_one(&db)
                .await
                .unwrap();
        let server =
            crate::webauthn::build_server("identity.example", "https://identity.example").unwrap();
        let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let started = registration::start(&db, &server, &token).await.unwrap();
        let response = authenticator
            .do_registration("https://identity.example".parse().unwrap(), started.options)
            .unwrap();
        let credential =
            registration::finish(&db, &server, &token, started.ceremony_id, &response, "Test")
                .await
                .unwrap();
        let clients = test_fixtures::clients();
        let handle = test_fixtures::request(&db, &clients, store::RequestKind::Authorization).await;
        let browser = random_secret();
        let started = interactions::begin(&db, &clients, &handle, &browser, None)
            .await
            .unwrap();
        let proof = test_fixtures::browser_proof(&browser, &started.csrf_token, None);
        Self {
            db,
            server,
            clients,
            handle,
            proof,
            principal,
            credential,
            authenticator,
        }
    }

    async fn assertion(&mut self) -> (Uuid, PublicKeyCredential) {
        let started = start(
            &self.db,
            &self.server,
            &self.clients,
            &self.handle,
            &self.proof,
        )
        .await
        .unwrap();
        assert!(started.options.public_key.allow_credentials.is_empty());
        let credential: Vec<u8> = sqlx::query_scalar(
            "SELECT credential_id FROM identity_webauthn_credentials WHERE id=$1",
        )
        .bind(self.credential)
        .fetch_one(&self.db)
        .await
        .unwrap();
        // SoftPasskey has no credential-discovery UI. Supply the selected ID to
        // the test authenticator only; the server retains its original state.
        let mut options = serde_json::to_value(started.options).unwrap();
        options["publicKey"]["allowCredentials"] =
            serde_json::json!([{"type":"public-key","id":URL_SAFE_NO_PAD.encode(credential)}]);
        let response = self
            .authenticator
            .do_authentication(
                "https://identity.example".parse().unwrap(),
                serde_json::from_value(options).unwrap(),
            )
            .unwrap();
        let mut response = serde_json::to_value(response).unwrap();
        response["response"]["userHandle"] =
            URL_SAFE_NO_PAD.encode(self.principal.as_bytes()).into();
        (
            started.ceremony_id,
            serde_json::from_value(response).unwrap(),
        )
    }
    async fn finish(
        &self,
        id: Uuid,
        response: &PublicKeyCredential,
    ) -> Result<AuthenticatedLogin, StoreError> {
        finish(
            &self.db,
            &self.server,
            &self.clients,
            &self.handle,
            &self.proof,
            id,
            response,
        )
        .await
    }
}

#[tokio::test]
async fn verified_passkey_creates_passwordless_session_and_rotates_interaction_csrf() {
    let mut f = Fixture::new().await;
    let (id, response) = f.assertion().await;
    let authenticated = f.finish(id, &response).await.unwrap();
    let row: (Uuid, String) = sqlx::query_as(
        "SELECT principal_id,primary_amr FROM identity_sessions WHERE token_hash=$1",
    )
    .bind(hash(&authenticated.token))
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(row, (f.principal, "webauthn".into()));
    let binding: (Uuid, Uuid, String) = sqlx::query_as("SELECT b.credential_id,b.principal_id,b.purpose FROM identity_session_webauthn_credentials b JOIN identity_sessions s ON s.id=b.session_id WHERE s.token_hash=$1")
        .bind(hash(&authenticated.token)).fetch_one(&f.db).await.unwrap();
    assert_eq!(binding, (f.credential, f.principal, "primary".into()));
    assert_ne!(authenticated.csrf, f.proof.csrf_token);
    assert!(f.finish(id, &response).await.is_err());
    let proof = test_fixtures::browser_proof(
        &f.proof.browser_token,
        &authenticated.csrf,
        Some(&authenticated.token),
    );
    let code = super::super::consent::approve(&f.db, &f.clients, &f.handle, &proof)
        .await
        .unwrap();
    assert!(!code.code.is_empty());
}

#[tokio::test]
async fn account_handle_and_browser_substitutions_do_not_authenticate() {
    let mut f = Fixture::new().await;
    let (id, response) = f.assertion().await;
    let (other_session, _) = session(&f.db).await;
    let other: Uuid = sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
        .bind(other_session)
        .fetch_one(&f.db)
        .await
        .unwrap();
    let mut altered = serde_json::to_value(&response).unwrap();
    altered["response"]["userHandle"] = URL_SAFE_NO_PAD.encode(other.as_bytes()).into();
    assert!(
        f.finish(id, &serde_json::from_value(altered).unwrap())
            .await
            .is_err()
    );
    let proof = test_fixtures::browser_proof(&random_secret(), &f.proof.csrf_token, None);
    assert!(
        finish(
            &f.db, &f.server, &f.clients, &f.handle, &proof, id, &response
        )
        .await
        .is_err()
    );
    assert!(f.finish(id, &response).await.is_ok());
}

#[tokio::test]
async fn concurrent_passkey_login_creates_one_session() {
    let mut f = Fixture::new().await;
    let (id, response) = f.assertion().await;
    let (a, b) = tokio::join!(f.finish(id, &response), f.finish(id, &response));
    assert_ne!(a.is_ok(), b.is_ok());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_sessions WHERE primary_amr='webauthn'")
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn revoked_credentials_and_suspended_principals_cannot_finish() {
    let mut f = Fixture::new().await;
    let (id, response) = f.assertion().await;
    sqlx::query(
        "UPDATE identity_webauthn_credentials SET revoked_at=clock_timestamp() WHERE id=$1",
    )
    .bind(f.credential)
    .execute(&f.db)
    .await
    .unwrap();
    assert!(f.finish(id, &response).await.is_err());
    let mut f = Fixture::new().await;
    let (id, response) = f.assertion().await;
    sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=$1")
        .bind(f.principal)
        .execute(&f.db)
        .await
        .unwrap();
    assert!(f.finish(id, &response).await.is_err());
}

#[tokio::test]
async fn failed_oauth_binding_rolls_back_session_counter_and_challenge() {
    let mut f = Fixture::new().await;
    let (id, response) = f.assertion().await;
    let before: serde_json::Value =
        sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE id=$1")
            .bind(f.credential)
            .fetch_one(&f.db)
            .await
            .unwrap();
    sqlx::query("ALTER TABLE identity_oauth_requests ADD CONSTRAINT injected_binding_failure CHECK (bound_session_id IS NULL)").execute(&f.db).await.unwrap();
    assert!(matches!(
        f.finish(id, &response).await,
        Err(StoreError::Database(_))
    ));
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_sessions WHERE primary_amr='webauthn'")
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let bindings: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_session_webauthn_credentials")
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(bindings, 0);
    let after: serde_json::Value =
        sqlx::query_scalar("SELECT passkey FROM identity_webauthn_credentials WHERE id=$1")
            .bind(f.credential)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(before, after);
    sqlx::query("ALTER TABLE identity_oauth_requests DROP CONSTRAINT injected_binding_failure")
        .execute(&f.db)
        .await
        .unwrap();
    assert!(f.finish(id, &response).await.is_ok());
}

#[tokio::test]
async fn revoking_primary_login_key_ends_its_session_even_with_another_key_enrolled() {
    let mut f = Fixture::new().await;
    let (id, response) = f.assertion().await;
    let authenticated = f.finish(id, &response).await.unwrap();
    let started = registration::start(&f.db, &f.server, &authenticated.token)
        .await
        .unwrap();
    let mut second = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let response = second
        .do_registration("https://identity.example".parse().unwrap(), started.options)
        .unwrap();
    registration::finish(
        &f.db,
        &f.server,
        &authenticated.token,
        started.ceremony_id,
        &response,
        "Spare key",
    )
    .await
    .unwrap();
    crate::webauthn::credentials::revoke(&f.db, &authenticated.token, f.credential)
        .await
        .unwrap();
    let revoked: bool = sqlx::query_scalar(
        "SELECT revoked_at IS NOT NULL FROM identity_sessions WHERE token_hash=$1",
    )
    .bind(hash(&authenticated.token))
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert!(revoked);
    assert!(
        registration::start(&f.db, &f.server, &authenticated.token)
            .await
            .is_err()
    );
}
