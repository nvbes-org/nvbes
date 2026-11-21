use super::super::{TokenService, tests::config};
use crate::{
    oauth::{
        clients::ClientRegistry,
        codes,
        http::token_router,
        store::{self, RequestKind},
    },
    test_fixtures::{self, authorize, database, exchange, session},
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use nvbes_dpop::{DpopKeyPair, generate_key_pair, proof::create_dpop_proof};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

struct Fixture {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    service: Arc<TokenService>,
    key: DpopKeyPair,
    refresh: String,
    session: uuid::Uuid,
    limiter: crate::rate_limits::RateLimiter,
}

impl Fixture {
    async fn new(bound: bool) -> Self {
        let db = database().await;
        let mut client =
            serde_json::to_value(test_fixtures::clients().get("account-web").unwrap()).unwrap();
        client["require_dpop"] = bound.into();
        let mut other = client.clone();
        other["client_id"] = "another-web".into();
        let clients = Arc::new(
            ClientRegistry::from_json(&serde_json::json!([client, other]).to_string(), false)
                .unwrap(),
        );
        let service = Arc::new(TokenService::new(config()).unwrap());
        let key = generate_key_pair();
        let mut input = test_fixtures::input();
        input.scope.push_str(" offline_access");
        input.dpop_jkt = bound.then(|| key.jkt.clone());
        let request = input.validate(&clients).unwrap();
        let handle = store::create_request(&db, &request, RequestKind::Authorization)
            .await
            .unwrap();
        let (session, cookie) = session(&db).await;
        let code = authorize(&db, &clients, &handle, &cookie).await.unwrap();
        let mut input = exchange(&code.code);
        input.verified_dpop_jkt = bound.then_some(key.jkt.as_str());
        let grant = codes::exchange(&db, &clients, input).await.unwrap();
        let refresh = service
            .issue_grant(&db, &clients, &grant)
            .await
            .unwrap()
            .refresh_token
            .unwrap();
        Self {
            db,
            clients,
            service,
            key,
            refresh,
            session,
            limiter: crate::rate_limits::RateLimiter::new(rand::random()).unwrap(),
        }
    }

    fn app(&self) -> Router {
        token_router(
            self.db.clone(),
            self.clients.clone(),
            self.service.clone(),
            self.limiter.clone(),
        )
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
}

async fn refresh(
    app: Router,
    secret: &str,
    client: Option<&str>,
    proofs: &[&str],
) -> (StatusCode, serde_json::Value) {
    let mut body = format!("grant_type=refresh_token&refresh_token={secret}");
    if let Some(client) = client {
        body.push_str(&format!("&client_id={client}"));
    }
    let mut request = Request::post("/oauth/token")
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:4000".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .header("content-type", "application/x-www-form-urlencoded");
    for proof in proofs {
        request = request.header("dpop", *proof);
    }
    let response = app
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["pragma"], "no-cache");
    let status = response.status();
    let body = to_bytes(response.into_body(), 32_768).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn refresh_http_requires_the_registered_client_for_bearer_tokens() {
    let f = Fixture::new(false).await;
    for (client, expected) in [
        (None, "invalid_request"),
        (Some("unknown-web"), "invalid_client"),
        (Some("another-web"), "invalid_grant"),
    ] {
        let (status, body) = refresh(f.app(), &f.refresh, client, &[]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], expected);
    }
    let (status, body) = refresh(f.app(), &f.refresh, Some("account-web"), &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["token_type"], "Bearer");
    assert_ne!(body["refresh_token"], f.refresh);
}

#[tokio::test]
async fn refresh_http_checks_key_method_uri_and_unique_header_before_rotation() {
    let f = Fixture::new(true).await;
    let other_key = create_dpop_proof(
        &generate_key_pair(),
        "POST",
        &f.service.endpoint("oauth/token"),
        None,
        None,
    )
    .unwrap();
    let wrong_method = create_dpop_proof(
        &f.key,
        "GET",
        &f.service.endpoint("oauth/token"),
        None,
        None,
    )
    .unwrap();
    let wrong_uri = create_dpop_proof(
        &f.key,
        "POST",
        "https://attacker.example/oauth/token",
        None,
        None,
    )
    .unwrap();
    let valid = f.proof();
    for proofs in [
        vec![],
        vec![other_key.as_str()],
        vec![wrong_method.as_str()],
        vec![wrong_uri.as_str()],
        vec!["invalid.jwt"],
        vec![""],
        vec![valid.as_str(), valid.as_str()],
    ] {
        let (status, body) = refresh(f.app(), &f.refresh, Some("account-web"), &proofs).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "invalid_dpop_proof");
    }
    // None of the rejected requests may consume the refresh secret or this proof.
    let (status, body) = refresh(f.app(), &f.refresh, Some("account-web"), &[&valid]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["token_type"], "DPoP");
    let claims = f
        .service
        .verify(
            body["access_token"].as_str().unwrap(),
            "nvbes-account-service",
        )
        .unwrap();
    assert_eq!(claims.cnf.unwrap().jkt, f.key.jkt);
    assert_eq!(claims.client_id, "account-web");
}

#[tokio::test]
async fn proof_replay_does_not_rotate_but_authenticated_refresh_replay_revokes_family() {
    let f = Fixture::new(true).await;
    let proof = f.proof();
    let (status, first) = refresh(f.app(), &f.refresh, Some("account-web"), &[&proof]).await;
    assert_eq!(status, StatusCode::OK);
    let next = first["refresh_token"].as_str().unwrap();
    let (status, error) = refresh(f.app(), next, Some("account-web"), &[&proof]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "invalid_dpop_proof");
    // A wrong client or a stolen secret without its key cannot revoke the family.
    assert_eq!(
        refresh(f.app(), &f.refresh, Some("another-web"), &[&f.proof()])
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        refresh(f.app(), &f.refresh, Some("account-web"), &[])
            .await
            .1["error"],
        "invalid_dpop_proof"
    );
    let (status, second) = refresh(f.app(), next, Some("account-web"), &[&f.proof()]).await;
    assert_eq!(status, StatusCode::OK);
    let (status, error) = refresh(f.app(), &f.refresh, Some("account-web"), &[&f.proof()]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "invalid_grant");
    assert_eq!(
        refresh(
            f.app(),
            second["refresh_token"].as_str().unwrap(),
            Some("account-web"),
            &[&f.proof()]
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert!(
        f.service
            .introspect(
                &f.db,
                &f.clients,
                second["access_token"].as_str().unwrap(),
                "nvbes-account-service"
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn concurrent_refreshes_serialize_and_replay_revokes_the_winner() {
    let f = Fixture::new(true).await;
    let a = f.proof();
    let b = f.proof();
    let a_headers = [a.as_str()];
    let b_headers = [b.as_str()];
    let results = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            refresh(f.app(), &f.refresh, Some("account-web"), &a_headers),
            refresh(f.app(), &f.refresh, Some("account-web"), &b_headers)
        )
    })
    .await
    .expect("refreshes must not deadlock");
    let ((a, av), (b, bv)) = results;
    assert_ne!(a == StatusCode::OK, b == StatusCode::OK);
    let (winner, loser) = if a == StatusCode::OK {
        (av, bv)
    } else {
        (bv, av)
    };
    assert_eq!(loser["error"], "invalid_grant");
    assert!(
        f.service
            .introspect(
                &f.db,
                &f.clients,
                winner["access_token"].as_str().unwrap(),
                "nvbes-account-service"
            )
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn failed_signing_rolls_back_proof_consumption_and_refresh_rotation() {
    let f = Fixture::new(true).await;
    let mut bad_config = config();
    bad_config.allowed_audiences.remove("nvbes-account-service");
    let bad_service = Arc::new(TokenService::new(bad_config).unwrap());
    let app = token_router(
        f.db.clone(),
        f.clients.clone(),
        bad_service.clone(),
        f.limiter.clone(),
    );
    let proof = f.proof();
    assert_eq!(
        refresh(app, &f.refresh, Some("account-web"), &[&proof])
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        refresh(f.app(), &f.refresh, Some("account-web"), &[&proof])
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn replay_of_previous_generation_cannot_deadlock_current_rotation() {
    let f = Fixture::new(true).await;
    let (_, issued) = refresh(f.app(), &f.refresh, Some("account-web"), &[&f.proof()]).await;
    let current = issued["refresh_token"].as_str().unwrap();
    let old_proof = f.proof();
    let current_proof = f.proof();
    let old_headers = [old_proof.as_str()];
    let current_headers = [current_proof.as_str()];
    let (old, current) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            refresh(f.app(), &f.refresh, Some("account-web"), &old_headers),
            refresh(f.app(), current, Some("account-web"), &current_headers)
        )
    })
    .await
    .expect("different refresh generations must share the same lock order");
    assert_eq!(old.0, StatusCode::BAD_REQUEST);
    assert_eq!(old.1["error"], "invalid_grant");
    if current.0 == StatusCode::OK {
        assert!(
            f.service
                .introspect(
                    &f.db,
                    &f.clients,
                    current.1["access_token"].as_str().unwrap(),
                    "nvbes-account-service"
                )
                .await
                .unwrap()
                .is_none()
        );
    } else {
        assert_eq!(current.0, StatusCode::BAD_REQUEST);
        assert_eq!(current.1["error"], "invalid_grant");
    }
}

#[tokio::test]
async fn refresh_refuses_expired_or_revoked_sessions_and_reports_store_failure() {
    for mutation in [
        "UPDATE identity_sessions SET created_at=clock_timestamp()-interval '2 hours',authenticated_at=clock_timestamp()-interval '2 hours',expires_at=clock_timestamp()-interval '1 second' WHERE id=$1",
        "UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1",
        "UPDATE identity_principals SET status='suspended' WHERE id=(SELECT principal_id FROM identity_sessions WHERE id=$1)",
    ] {
        let f = Fixture::new(true).await;
        sqlx::query(mutation)
            .bind(f.session)
            .execute(&f.db)
            .await
            .unwrap();
        let (status, body) = refresh(f.app(), &f.refresh, Some("account-web"), &[&f.proof()]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "invalid_grant");
    }
    let f = Fixture::new(true).await;
    f.db.close().await;
    let (status, body) = refresh(f.app(), &f.refresh, Some("account-web"), &[&f.proof()]).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "temporarily_unavailable");
}
