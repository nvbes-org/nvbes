use super::{ACCESS_TOKEN_TTL_SECONDS, TokenService};
use crate::{
    authentication::Authentication, oauth::clients::ClientRegistry, tokens_claims::IdTokenClaims,
    tokens_config::TokenConfig, tokens_grants::ActiveGrant,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, Validation, decode, decode_header};
use rsa::{
    RsaPrivateKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    rand_core::OsRng,
};
use std::sync::OnceLock;
use uuid::Uuid;

pub(crate) fn config() -> TokenConfig {
    static CONFIG: OnceLock<TokenConfig> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let private = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
            TokenConfig::from_values(
                "test",
                "http://localhost:3000".into(),
                "identity-key-1".into(),
                private
                    .to_pkcs8_pem(Default::default())
                    .unwrap()
                    .to_string(),
                private
                    .to_public_key()
                    .to_public_key_pem(Default::default())
                    .unwrap(),
                "nvbes-account-service,nvbes-billing-service".into(),
            )
            .unwrap()
        })
        .clone()
}

fn grant() -> ActiveGrant {
    let clients = ClientRegistry::from_json(r#"[{
        "client_id":"account-web","display_name":"Account",
        "redirect_uris":["https://account.example/callback"],"post_logout_redirect_uris":[],
        "resources":{"https://api.example/account":{"audience":"nvbes-account-service","scopes":["account:read"]}},
        "allow_refresh":false,"require_dpop":false
    }]"#, false).unwrap();
    let request = serde_json::from_value::<crate::oauth::request::AuthorizationInput>(serde_json::json!({
        "client_id":"account-web","redirect_uri":"https://account.example/callback","response_type":"code",
        "scope":"openid account:read","resource":"https://api.example/account",
        "state":"transaction-state-123","nonce":"transaction-nonce-123",
        "code_challenge":"E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM","code_challenge_method":"S256"
    })).unwrap().validate(&clients).unwrap();
    ActiveGrant {
        id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        principal_id: Uuid::new_v4(),
        request,
        authentication: Authentication {
            authenticated_at: Utc::now() - Duration::minutes(2),
            primary_amr: "pwd".into(),
            step_up_method: None,
            step_up_at: None,
            step_up_expires_at: None,
        },
        session_expires_at: Utc::now() + Duration::hours(1),
        token_issued_at: None,
    }
}

<<<<<<< HEAD
#[test]
fn oidc_and_api_tokens_have_distinct_audiences_and_types() {
    let service = TokenService::new(config()).unwrap();
    let grant = grant();
    let response = service.sign_grant(&grant).unwrap();
    let header = decode_header(&response.access_token).unwrap();
    assert_eq!(header.alg, Algorithm::RS256);
    assert_eq!(header.typ.as_deref(), Some("at+jwt"));
    let access = service
        .verify(&response.access_token, "nvbes-account-service")
        .unwrap();
    assert_eq!(access.exp - access.iat, ACCESS_TOKEN_TTL_SECONDS);
    assert_eq!(access.grant_id, grant.id.to_string());
    assert_eq!(access.scope, "account:read");
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&["account-web"]);
    validation.set_issuer(&[service.issuer()]);
    let identity = decode::<IdTokenClaims>(
        &response.id_token,
        service
            .keys
            .decoding_key("identity-key-1", Utc::now().timestamp() as u64)
            .unwrap(),
        &validation,
=======
fn service() -> TokenService {
    let private = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    TokenService::new(
        TokenConfig::from_values(
            "test",
            "http://identity.test".into(),
            "identity-key-1".into(),
            String::from_utf8(private.private_key_to_pem_pkcs8().unwrap()).unwrap(),
            String::from_utf8(private.public_key_to_pem().unwrap()).unwrap(),
            "nvbes-account-service".into(),
        )
        .unwrap(),
>>>>>>> origin/main
    )
    .unwrap()
    .claims;
    assert_eq!(identity.sub, access.sub);
    assert_eq!(
        identity.auth_time,
        grant.authentication.authenticated_at.timestamp() as u64
    );
    assert_eq!(identity.nonce, "transaction-nonce-123");
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};
    assert_eq!(
        identity.at_hash,
        URL_SAFE_NO_PAD.encode(&Sha256::digest(response.access_token.as_bytes())[..16])
    );
    assert!(
        service
            .verify(&response.id_token, "nvbes-account-service")
            .is_err()
    );
    assert!(
        service
            .verify(&response.access_token, "nvbes-billing-service")
            .is_err()
    );
    assert!(
        service
            .verify(&response.access_token, "account-web")
            .is_err()
    );
}

#[test]
fn passwordless_and_step_up_evidence_are_not_invented() {
    let service = TokenService::new(config()).unwrap();
    let mut grant = grant();
    grant.authentication.primary_amr = "webauthn".into();
    let response = service.sign_grant(&grant).unwrap();
    let access = service
        .verify(&response.access_token, "nvbes-account-service")
        .unwrap();
    assert_eq!(access.amr, ["webauthn"]);
    assert!(access.step_up_time.is_none());
    grant.authentication.step_up_method = Some("totp".into());
    assert!(
        service.sign_grant(&grant).is_err(),
        "a method without its proof time is rejected"
    );
    grant.authentication.step_up_at = Some(Utc::now() - Duration::minutes(1));
    grant.authentication.step_up_expires_at = Some(Utc::now() + Duration::minutes(5));
    let response = service.sign_grant(&grant).unwrap();
    let access = service
        .verify(&response.access_token, "nvbes-account-service")
        .unwrap();
    assert_eq!(access.amr, ["webauthn", "totp"]);
    assert!(access.step_up_time.is_some());
    grant.authentication.step_up_expires_at = Some(Utc::now() - Duration::seconds(1));
    let response = service.sign_grant(&grant).unwrap();
    let access = service
        .verify(&response.access_token, "nvbes-account-service")
        .unwrap();
    assert_eq!(access.amr, ["webauthn"]);
    assert!(access.step_up_time.is_none());
}

#[test]
fn hostile_claims_and_scope_escalation_fail_validation() {
    let service = TokenService::new(config()).unwrap();
    let mut grant = grant();
    let response = service.sign_grant(&grant).unwrap();
    let access = service
        .verify(&response.access_token, "nvbes-account-service")
        .unwrap();
    for field in ["sub", "sid", "grant_id", "jti"] {
        let mut value = serde_json::to_value(&access).unwrap();
        value[field] = serde_json::json!("invalid");
        let token = service.keys.sign("at+jwt", &value).unwrap();
        assert!(service.verify(&token, "nvbes-account-service").is_err());
    }
    for field in ["iat", "nbf", "auth_time"] {
        let mut value = serde_json::to_value(&access).unwrap();
        value[field] = serde_json::json!(access.exp + 100);
        assert!(
            service
                .verify(
                    &service.keys.sign("at+jwt", &value).unwrap(),
                    "nvbes-account-service"
                )
                .is_err()
        );
    }
    grant.request.scope = "openid billing:checkout".into();
    assert!(service.sign_grant(&grant).is_err());
    grant.request.scope = "openid account:read".into();
    grant.session_expires_at = Utc::now() - Duration::seconds(1);
    assert!(service.sign_grant(&grant).is_err());
}

#[test]
fn overlapping_keys_verify_until_retirement_and_only_active_key_signs() {
    let old = config();
    let old_service = TokenService::new(old.clone()).unwrap();
    let old_token = old_service.sign_grant(&grant()).unwrap().access_token;
    let private = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let mut rotated = old.clone();
    rotated.key_id = "identity-key-2".into();
    rotated.private_key_pem = private
        .to_pkcs8_pem(Default::default())
        .unwrap()
        .to_string();
    rotated.public_key_pem = private
        .to_public_key()
        .to_public_key_pem(Default::default())
        .unwrap();
    let overlap = serde_json::json!([{"kid":old.key_id,"public_key_pem":old.public_key_pem,
        "accept_until":Utc::now().timestamp() as u64+3600}]);
    let current = TokenService::new(
        rotated
            .clone()
            .with_verification_keys(&overlap.to_string())
            .unwrap(),
    )
    .unwrap();
    assert!(current.verify(&old_token, "nvbes-account-service").is_ok());
    assert_eq!(current.jwks().keys.len(), 2);
    let issued = current.sign_grant(&grant()).unwrap().access_token;
    assert_eq!(
        decode_header(&issued).unwrap().kid.as_deref(),
        Some("identity-key-2")
    );
    let mut expired = overlap;
    expired[0]["accept_until"] = serde_json::json!(1);
    let retired = TokenService::new(
        rotated
            .with_verification_keys(&expired.to_string())
            .unwrap(),
    )
    .unwrap();
    assert!(retired.verify(&old_token, "nvbes-account-service").is_err());
    assert_eq!(retired.jwks().keys.len(), 1);
}

#[test]
fn mismatched_or_weak_keys_fail_startup() {
    let mut mismatch = config();
    let weak = RsaPrivateKey::new(&mut OsRng, 1024).unwrap();
    mismatch.private_key_pem = weak.to_pkcs8_pem(Default::default()).unwrap().to_string();
    assert!(TokenService::new(mismatch.clone()).is_err());
    mismatch.public_key_pem = weak
        .to_public_key()
        .to_public_key_pem(Default::default())
        .unwrap();
    assert!(TokenService::new(mismatch).is_err());
}
