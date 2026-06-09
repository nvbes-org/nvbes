use axum::{Json, Router, routing::get};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use openssl::rsa::Rsa;
use serde_json::json;
use tokio::net::TcpListener;
use uuid::Uuid;

use super::{fetch_oidc_discovery, validate_oidc_id_token};
use crate::domains::federation::types::FederatedIdentityProviderRecord;

async fn spawn_server(build_router: impl FnOnce(String) -> Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{}", addr);
    let router = build_router(base_url.clone());
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("test server should run");
    });
    base_url
}

fn provider(base_url: &str) -> FederatedIdentityProviderRecord {
    FederatedIdentityProviderRecord {
        id: Uuid::new_v4(),
        provider_type: "oidc".to_string(),
        name: "Acme OIDC".to_string(),
        client_id: Some("client-123".to_string()),
        issuer: Some(format!("{}/issuer", base_url)),
        metadata_url: Some(format!("{}/.well-known/openid-configuration", base_url)),
        status: "active".to_string(),
        sp_entity_id: None,
        attribute_mapping: json!({}),
        encryption_cert_pem: None,
        require_signed_assertions: true,
        require_signed_responses: true,
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn fetch_oidc_discovery_parses_standard_document() {
    let base_url = spawn_server(|_| {
        Router::new().route(
            "/.well-known/openid-configuration",
            get(|| async {
                Json(json!({
                    "issuer": "https://issuer.example.com",
                    "jwks_uri": "https://issuer.example.com/jwks",
                    "authorization_endpoint": "https://issuer.example.com/authorize",
                    "token_endpoint": "https://issuer.example.com/token",
                    "userinfo_endpoint": "https://issuer.example.com/userinfo",
                    "response_types_supported": ["code"],
                    "subject_types_supported": ["public"],
                    "id_token_signing_alg_values_supported": ["RS256"],
                    "claims_supported": ["sub", "email", "preferred_username"],
                    "scopes_supported": ["openid", "profile", "email"]
                }))
            }),
        )
    })
    .await;

    let provider = provider(&base_url);

    let document = fetch_oidc_discovery(&provider, false)
        .await
        .expect("discovery document should parse");

    assert_eq!(document.issuer, "https://issuer.example.com");
    assert_eq!(
        document.jwks_uri.as_deref(),
        Some("https://issuer.example.com/jwks")
    );
}

#[tokio::test]
async fn validate_oidc_id_token_accepts_real_rs256_token() {
    let rsa = Rsa::generate(2048).expect("RSA key should generate");
    let private_key_pem = rsa.private_key_to_pem().expect("private key PEM");
    let public_n = URL_SAFE_NO_PAD.encode(rsa.n().to_vec());
    let public_e = URL_SAFE_NO_PAD.encode(rsa.e().to_vec());
    let kid = "interop-kid";

    let base_url = spawn_server(move |base_url| {
        Router::new()
            .route(
                "/.well-known/openid-configuration",
                get({
                    let base_url = base_url.clone();
                    move || {
                        let base_url = base_url.clone();
                        async move {
                            Json(json!({
                                "issuer": format!("{}/issuer", base_url),
                                "jwks_uri": format!("{}/jwks", base_url),
                                "response_types_supported": ["code"],
                                "subject_types_supported": ["public"],
                                "id_token_signing_alg_values_supported": ["RS256"],
                                "claims_supported": ["sub", "email", "preferred_username"],
                                "scopes_supported": ["openid", "profile", "email"]
                            }))
                        }
                    }
                }),
            )
            .route(
                "/jwks",
                get({
                    let kid = kid.to_string();
                    let public_n = public_n.clone();
                    let public_e = public_e.clone();
                    move || {
                        let kid = kid.clone();
                        let public_n = public_n.clone();
                        let public_e = public_e.clone();
                        async move {
                            Json(json!({
                                "keys": [{
                                    "kid": kid,
                                    "kty": "RSA",
                                    "alg": "RS256",
                                    "use": "sig",
                                    "n": public_n,
                                    "e": public_e
                                }]
                            }))
                        }
                    }
                }),
            )
    })
    .await;

    let provider = provider(&base_url);
    let claims = json!({
        "iss": format!("{}/issuer", base_url),
        "sub": "subject-123",
        "aud": "client-123",
        "exp": (Utc::now() + chrono::Duration::minutes(5)).timestamp(),
        "nonce": "nonce-456",
        "email": "alice@example.com",
        "email_verified": true,
        "preferred_username": "alice"
    });
    let token = encode(
        &Header {
            alg: Algorithm::RS256,
            kid: Some(kid.to_string()),
            ..Header::default()
        },
        &claims,
        &EncodingKey::from_rsa_pem(&private_key_pem).expect("encoding key"),
    )
    .expect("token should encode");

    let validated = validate_oidc_id_token(&provider, &token, Some("nonce-456"), false)
        .await
        .expect("token should validate");

    assert_eq!(validated.sub, "subject-123");
    assert_eq!(validated.email.as_deref(), Some("alice@example.com"));
    assert_eq!(validated.username(), "alice");
    assert_eq!(validated.iss, format!("{}/issuer", base_url));
    assert!(validated.audience_contains("client-123"));
    assert_eq!(validated.nonce.as_deref(), Some("nonce-456"));
}
