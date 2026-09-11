use super::*;
use crate::{
    oauth::{
        http::token_router,
        store::{self, RequestKind},
    },
    test_fixtures::{self, authorize, database, session},
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use nvbes_dpop::{DpopKeyPair, generate_key_pair, proof::create_dpop_proof};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tower::ServiceExt;

struct Fixture {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    service: Arc<TokenService>,
    key: DpopKeyPair,
    code: String,
    session: uuid::Uuid,
    principal: uuid::Uuid,
    limiter: crate::rate_limits::RateLimiter,
}

impl Fixture {
    async fn new() -> Self {
        let db = database().await;
        let key = generate_key_pair();
        let mut client =
            serde_json::to_value(test_fixtures::clients().get("account-web").unwrap()).unwrap();
        client["require_dpop"] = true.into();
        let clients = Arc::new(
            ClientRegistry::from_json(&serde_json::json!([client]).to_string(), false).unwrap(),
        );
        let mut input = test_fixtures::input();
        input.dpop_jkt = Some(key.jkt.clone());
        input.scope.push_str(" offline_access");
        let handle = store::create_request(
            &db,
            &input.validate(&clients).unwrap(),
            RequestKind::Authorization,
        )
        .await
        .unwrap();
        let (session, cookie) = session(&db).await;
        let principal =
            sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
                .bind(session)
                .fetch_one(&db)
                .await
                .unwrap();
        let code = authorize(&db, &clients, &handle, &cookie)
            .await
            .unwrap()
            .code;
        Self {
            db,
            clients,
            service: Arc::new(TokenService::new(crate::tokens::tests::config()).unwrap()),
            key,
            code,
            session,
            principal,
            limiter: crate::rate_limits::RateLimiter::new(rand::random()).unwrap(),
        }
    }
    fn proof(&self) -> String {
        create_dpop_proof(
            &self.key,
            "POST",
            &self.service.endpoint("oauth/token"),
            None,
            None,
        )
        .unwrap()
    }
    fn app(&self, service: Arc<TokenService>) -> Router {
        token_router(
            self.db.clone(),
            self.clients.clone(),
            service.clone(),
            self.limiter.clone(),
        )
    }
    async fn submit(
        &self,
        service: Arc<TokenService>,
        proof: Option<&str>,
        verifier: &str,
    ) -> (StatusCode, serde_json::Value) {
        let mut form = reqwest::Url::parse("https://identity.invalid/").unwrap();
        for (key, value) in [
            ("grant_type", "authorization_code"),
            ("code", self.code.as_str()),
            ("client_id", "account-web"),
            ("redirect_uri", "https://account.example/callback"),
            ("code_verifier", verifier),
        ] {
            form.query_pairs_mut().append_pair(key, value);
        }
        let mut request = Request::post("/oauth/token")
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:4500".parse::<std::net::SocketAddr>().unwrap(),
            ))
            .header("content-type", "application/x-www-form-urlencoded");
        if let Some(proof) = proof {
            request = request.header("dpop", proof);
        }
        let response = self
            .app(service)
            .oneshot(
                request
                    .body(Body::from(form.query().unwrap().to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        let status = response.status();
        (
            status,
            serde_json::from_slice(&to_bytes(response.into_body(), 32768).await.unwrap()).unwrap(),
        )
    }
    async fn state(&self) -> (bool, i64, i64, i64) {
        sqlx::query_as("SELECT consumed_at IS NOT NULL,(SELECT count(*) FROM identity_oauth_grants WHERE code_hash=c.code_hash),(SELECT count(*) FROM identity_dpop_replay_keys WHERE jkt_hash=$2),(SELECT count(*) FROM identity_audit_events WHERE principal_id=$3 AND event_type IN ('identity.oauth.code_exchanged','identity.oauth.tokens_issued','identity.oauth.code_replayed')) FROM identity_oauth_codes c WHERE code_hash=$1")
            .bind(store::hash(&self.code)).bind(Sha256::digest(self.key.jkt.as_bytes()).to_vec()).bind(self.principal).fetch_one(&self.db).await.unwrap()
    }
}

fn verifier() -> String {
    test_fixtures::exchange("").verifier.to_owned()
}

#[tokio::test]
async fn refresh_insert_failure_rolls_back_the_entire_exchange() {
    let f = Fixture::new().await;
    let name = format!("fail_exchange_{}", f.principal.simple());
    // The trigger only rejects this fixture's principal in the dedicated test DB.
    sqlx::query(&format!("CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.principal_id='{}'::uuid THEN RAISE EXCEPTION 'injected refresh failure'; END IF; RETURN NEW; END $$",f.principal))
        .execute(&f.db).await.unwrap();
    sqlx::query(&format!("CREATE TRIGGER {name} BEFORE INSERT ON identity_oauth_refresh_tokens FOR EACH ROW EXECUTE FUNCTION {name}()"))
        .execute(&f.db).await.unwrap();
    let proof = f.proof();
    let response = f.submit(f.service.clone(), Some(&proof), &verifier()).await;
    sqlx::query(&format!("DROP FUNCTION {name}() CASCADE"))
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(response.0, StatusCode::SERVICE_UNAVAILABLE);
    assert!(response.1.get("access_token").is_none());
    assert_eq!(f.state().await, (false, 0, 0, 0));
    assert_eq!(
        f.submit(f.service.clone(), Some(&proof), &verifier())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(f.state().await, (true, 1, 1, 2));
}

#[tokio::test]
async fn simultaneous_fresh_proofs_commit_replay_revocation_without_deadlock() {
    let f = Fixture::new().await;
    let a = f.proof();
    let b = f.proof();
    let verifier = verifier();
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            f.submit(f.service.clone(), Some(&a), &verifier),
            f.submit(f.service.clone(), Some(&b), &verifier)
        )
    })
    .await
    .unwrap();
    assert_ne!(a.0 == StatusCode::OK, b.0 == StatusCode::OK);
    let winner = if a.0 == StatusCode::OK { a } else { b };
    assert_eq!(f.state().await, (true, 1, 2, 3));
    assert!(
        f.service
            .introspect(
                &f.db,
                &f.clients,
                winner.1["access_token"].as_str().unwrap(),
                "nvbes-account-service"
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn signing_failure_rolls_back_code_grant_proof_and_audit_before_retry() {
    let f = Fixture::new().await;
    let proof = f.proof();
    let mut config = crate::tokens::tests::config();
    config.allowed_audiences.remove("nvbes-account-service");
    let bad = Arc::new(TokenService::new(config).unwrap());
    assert_eq!(
        f.submit(bad, Some(&proof), &verifier()).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(f.state().await, (false, 0, 0, 0));
    let response = f.submit(f.service.clone(), Some(&proof), &verifier()).await;
    assert_eq!(response.0, StatusCode::OK);
    assert_eq!(response.1["token_type"], "DPoP");
    assert!(response.1["refresh_token"].is_string());
    assert_eq!(f.state().await, (true, 1, 1, 2));
}

#[tokio::test]
async fn only_fresh_authenticated_code_replay_revokes_issued_tokens() {
    let f = Fixture::new().await;
    let proof = f.proof();
    let issued = f.submit(f.service.clone(), Some(&proof), &verifier()).await;
    assert_eq!(issued.0, StatusCode::OK);
    for supplied in [None, Some(proof.as_str())] {
        assert_eq!(
            f.submit(f.service.clone(), supplied, &verifier()).await.0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(f.state().await, (true, 1, 1, 2));
        assert!(
            f.service
                .introspect(
                    &f.db,
                    &f.clients,
                    issued.1["access_token"].as_str().unwrap(),
                    "nvbes-account-service"
                )
                .await
                .unwrap()
                .is_some()
        );
    }
    assert_eq!(
        f.submit(f.service.clone(), Some(&f.proof()), &verifier())
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(f.state().await, (true, 1, 2, 3));
    assert!(
        f.service
            .introspect(
                &f.db,
                &f.clients,
                issued.1["access_token"].as_str().unwrap(),
                "nvbes-account-service"
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn rejected_pkce_or_revoked_session_does_not_consume_the_proof() {
    let f = Fixture::new().await;
    let proof = f.proof();
    assert_eq!(
        f.submit(f.service.clone(), Some(&proof), &"x".repeat(43))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(f.state().await, (false, 0, 0, 0));
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(f.session)
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.submit(f.service.clone(), Some(&proof), &verifier())
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(f.state().await, (false, 0, 0, 0));
}

#[tokio::test]
async fn simultaneous_identical_requests_issue_once_without_revoking_the_winner() {
    let f = Fixture::new().await;
    let proof = f.proof();
    let verifier = verifier();
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            f.submit(f.service.clone(), Some(&proof), &verifier),
            f.submit(f.service.clone(), Some(&proof), &verifier)
        )
    })
    .await
    .unwrap();
    assert_ne!(a.0 == StatusCode::OK, b.0 == StatusCode::OK);
    let winner = if a.0 == StatusCode::OK { a } else { b };
    assert_eq!(f.state().await, (true, 1, 1, 2));
    assert!(
        f.service
            .introspect(
                &f.db,
                &f.clients,
                winner.1["access_token"].as_str().unwrap(),
                "nvbes-account-service"
            )
            .await
            .unwrap()
            .is_some()
    );
}
