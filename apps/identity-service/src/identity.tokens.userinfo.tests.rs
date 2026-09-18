use super::*;
use crate::{
    oauth::{
        http::token_router,
        store::{self, RequestKind},
    },
    test_fixtures::{self, authorize, isolated_database, session},
    tokens::AuthorizationCodeRequest,
};
use axum::{
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
};
use nvbes_dpop::{DpopKeyPair, generate_key_pair, proof::create_dpop_proof};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tower::ServiceExt;

#[path = "identity.tokens.userinfo.boundaries.tests.rs"]
mod boundaries;

struct Fixture {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    service: Arc<TokenService>,
    key: DpopKeyPair,
    token: String,
    id_token: String,
    principal: uuid::Uuid,
    limiter: crate::rate_limits::RateLimiter,
}

impl Fixture {
    async fn new(scope: &str, bound: bool, audience: &str) -> Self {
        let db = isolated_database().await;
        let mut config = crate::tokens::tests::config();
        config
            .allowed_audiences
            .insert(tokens_policy::USERINFO_AUDIENCE.into());
        let service = Arc::new(TokenService::new(config).unwrap());
        let resource = service.endpoint("oauth/userinfo");
        let mut client =
            serde_json::to_value(test_fixtures::clients().get("account-web").unwrap()).unwrap();
        client["resources"][&resource] = serde_json::json!({"audience":tokens_policy::USERINFO_AUDIENCE,"scopes":["openid","profile","email"]});
        client["require_dpop"] = bound.into();
        let clients = Arc::new(
            ClientRegistry::from_json(&serde_json::json!([client]).to_string(), true).unwrap(),
        );
        let key = generate_key_pair();
        let mut input = test_fixtures::input();
        if audience == tokens_policy::USERINFO_AUDIENCE {
            input.resource = resource;
        }
        input.scope = scope.into();
        input.dpop_jkt = bound.then(|| key.jkt.clone());
        let handle = store::create_request(
            &db,
            &input.validate(&clients).unwrap(),
            RequestKind::Authorization,
        )
        .await
        .unwrap();
        let (session, cookie) = session(&db).await;
        let principal: uuid::Uuid =
            sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
                .bind(session)
                .fetch_one(&db)
                .await
                .unwrap();
        sqlx::query("INSERT INTO identity_login_identifiers(id,principal_id,kind,normalized_value,verified_at) VALUES($1,$2,'email',$3,clock_timestamp())")
            .bind(uuid::Uuid::new_v4()).bind(principal).bind(format!("{principal}@example.invalid")).execute(&db).await.unwrap();
        let code = authorize(&db, &clients, &handle, &cookie).await.unwrap();
        let proof = bound.then(|| {
            create_dpop_proof(&key, "POST", &service.endpoint("oauth/token"), None, None).unwrap()
        });
        let exchange = test_fixtures::exchange(&code.code);
        let response = service
            .exchange_code(
                &db,
                &clients,
                AuthorizationCodeRequest {
                    code: &code.code,
                    client_id: exchange.client_id,
                    redirect_uri: exchange.redirect_uri,
                    verifier: exchange.verifier,
                    dpop_proof: proof.as_deref(),
                },
            )
            .await
            .unwrap();
        Self {
            db,
            clients,
            service,
            key,
            token: response.access_token,
            id_token: response.id_token,
            principal,
            limiter: crate::rate_limits::RateLimiter::new(rand::random()).unwrap(),
        }
    }
    fn proof(&self, method: &str, token: Option<&str>, url: &str) -> String {
        create_dpop_proof(&self.key, method, url, token, None).unwrap()
    }
    fn endpoint(&self) -> String {
        self.service.endpoint("oauth/userinfo")
    }
    async fn request(
        &self,
        method: &str,
        token: &str,
        scheme: &str,
        proof: Option<&str>,
    ) -> (StatusCode, HeaderMap, serde_json::Value) {
        let mut request = Request::builder()
            .method(method)
            .uri("/oauth/userinfo")
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:6000".parse::<std::net::SocketAddr>().unwrap(),
            ))
            .header("authorization", format!("{scheme} {token}"));
        if let Some(proof) = proof {
            request = request.header("dpop", proof);
        }
        let app = token_router(
            self.db.clone(),
            self.clients.clone(),
            self.service.clone(),
            self.limiter.clone(),
        );
        let response = app
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        let status = response.status();
        let headers = response.headers().clone();
        (
            status,
            headers,
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap(),
        )
    }
}

#[tokio::test]
async fn userinfo_returns_only_granted_verified_claims_on_get_and_post() {
    let f = Fixture::new("openid email", false, tokens_policy::USERINFO_AUDIENCE).await;
    for method in ["GET", "POST"] {
        let result = f.request(method, &f.token, "bEaReR", None).await;
        assert_eq!(result.0, StatusCode::OK);
        assert_eq!(
            result.2,
            serde_json::json!({"sub":f.principal.to_string(),"email":format!("{}@example.invalid",f.principal),"email_verified":true})
        );
    }
    sqlx::query("UPDATE identity_login_identifiers SET verified_at=NULL WHERE principal_id=$1")
        .bind(f.principal)
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.request("GET", &f.token, "Bearer", None).await.2,
        serde_json::json!({"sub":f.principal.to_string()})
    );
    let f = Fixture::new("openid profile", false, tokens_policy::USERINFO_AUDIENCE).await;
    assert_eq!(
        f.request("GET", &f.token, "Bearer", None).await.2,
        serde_json::json!({"sub":f.principal.to_string()})
    );
}

#[tokio::test]
async fn wrong_audiences_id_tokens_and_suspended_accounts_are_rejected() {
    let f = Fixture::new(
        "openid account:read email",
        false,
        tokens_policy::ACCOUNT_AUDIENCE,
    )
    .await;
    assert_eq!(
        f.request("GET", &f.token, "Bearer", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let f = Fixture::new("openid email", false, tokens_policy::USERINFO_AUDIENCE).await;
    assert_eq!(
        f.request("GET", &f.id_token, "Bearer", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert!(
        f.service
            .verify(&f.token, tokens_policy::ACCOUNT_AUDIENCE)
            .is_err()
    );
    sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=$1")
        .bind(f.principal)
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.request("GET", &f.token, "Bearer", None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn dpop_requires_matching_scheme_key_method_uri_and_access_token_hash() {
    let f = Fixture::new("openid email", true, tokens_policy::USERINFO_AUDIENCE).await;
    let good = f.proof("GET", Some(&f.token), &f.endpoint());
    assert_eq!(
        f.request("GET", &f.token, "Bearer", Some(&good)).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.request("GET", &f.token, "DPoP", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let wrong_key = create_dpop_proof(
        &generate_key_pair(),
        "GET",
        &f.endpoint(),
        Some(&f.token),
        None,
    )
    .unwrap();
    for bad in [
        wrong_key,
        f.proof("POST", Some(&f.token), &f.endpoint()),
        f.proof("GET", Some(&f.token), "https://other.example/userinfo"),
        f.proof("GET", None, &f.endpoint()),
        f.proof("GET", Some("different-token"), &f.endpoint()),
    ] {
        assert_eq!(
            f.request("GET", &f.token, "DPoP", Some(&bad)).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        f.request("GET", &f.token, "dPoP", Some(&good)).await.0,
        StatusCode::OK
    );
    let replay = f.request("GET", &f.token, "DPoP", Some(&good)).await;
    assert_eq!(replay.0, StatusCode::UNAUTHORIZED);
    assert!(
        replay.1["www-authenticate"]
            .to_str()
            .unwrap()
            .starts_with("DPoP ")
    );
    assert_eq!(
        f.request(
            "POST",
            &f.token,
            "DPoP",
            Some(&f.proof("POST", Some(&f.token), &f.endpoint()))
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn concurrent_userinfo_proofs_are_consumed_once() {
    let f = Fixture::new("openid", true, tokens_policy::USERINFO_AUDIENCE).await;
    let proof = f.proof("GET", Some(&f.token), &f.endpoint());
    let (a, b) = tokio::join!(
        f.request("GET", &f.token, "DPoP", Some(&proof)),
        f.request("GET", &f.token, "DPoP", Some(&proof))
    );
    assert_ne!(a.0 == StatusCode::OK, b.0 == StatusCode::OK);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_dpop_replay_keys WHERE jkt_hash=$1")
            .bind(Sha256::digest(f.key.jkt.as_bytes()).to_vec())
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(count, 2); // Token endpoint plus one UserInfo request.
}
