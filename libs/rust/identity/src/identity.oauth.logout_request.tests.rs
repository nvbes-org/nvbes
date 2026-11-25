use super::*;
use crate::tokens::tests::config;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::json;

const RETURN: &str = "https://account.example/signed-out?source=identity";

fn clients() -> ClientRegistry {
    ClientRegistry::from_json(&json!([{
        "client_id":"account-web","display_name":"Account",
        "redirect_uris":["https://account.example/callback"],
        "post_logout_redirect_uris":[RETURN],
        "resources":{"https://api.example/account":{"audience":"nvbes-account-service","scopes":["account:read"]}},
        "allow_refresh":false,"require_dpop":false
    }]).to_string(), false).unwrap()
}

fn hint(client: &str) -> String {
    let config = config();
    let now = chrono::Utc::now().timestamp() as u64;
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(config.key_id);
    encode(
        &header,
        &json!({
            "iss":config.issuer,"aud":client,"sub":uuid::Uuid::new_v4(),
            "sid":uuid::Uuid::new_v4(),"iat":now-10,"exp":now+100,"auth_time":now-20,
            "amr":["pwd"],"nonce":"nonce","at_hash":"hash"
        }),
        &EncodingKey::from_rsa_pem(config.private_key_pem.as_bytes()).unwrap(),
    )
    .unwrap()
}

fn request(fields: &[(&str, &str)]) -> Result<LogoutRequest, OAuthError> {
    LogoutRequest::from_fields(
        fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        &clients(),
        &TokenService::new(config()).unwrap(),
    )
}

#[test]
fn signed_client_and_exact_registered_return_preserve_opaque_state() {
    let token = hint("account-web");
    let state = "opaque +&=é?next=https://attacker.example";
    let request = request(&[
        ("id_token_hint", &token),
        ("post_logout_redirect_uri", RETURN),
        ("state", state),
    ])
    .unwrap();
    assert_eq!(request.client_id.as_deref(), Some("account-web"));
    let target = reqwest::Url::parse(&request.return_uri().unwrap().unwrap()).unwrap();
    assert_eq!(
        target.origin().ascii_serialization(),
        "https://account.example"
    );
    assert_eq!(target.path(), "/signed-out");
    let pairs: BTreeMap<_, _> = target.query_pairs().collect();
    assert_eq!(pairs["state"], state);
    assert_eq!(pairs["source"], "identity");
}

#[test]
fn untrusted_redirects_and_client_confusion_are_rejected() {
    let token = hint("account-web");
    for uri in [
        "https://attacker.example",
        "https://account.example/signed-out",
        "https://account.example/signed-out?source=identity&extra=1",
        "https://ACCOUNT.example/signed-out?source=identity",
        "//account.example/signed-out",
    ] {
        assert!(request(&[("id_token_hint", &token), ("post_logout_redirect_uri", uri)]).is_err());
    }
    assert!(request(&[("id_token_hint", &token), ("client_id", "another-client")]).is_err());
    assert!(request(&[("id_token_hint", &hint("unknown-client"))]).is_err());
    assert!(
        request(&[
            ("client_id", "account-web"),
            ("post_logout_redirect_uri", RETURN)
        ])
        .is_err()
    );
    assert!(request(&[("post_logout_redirect_uri", RETURN)]).is_err());
}

#[test]
fn duplicate_and_oversized_parameters_never_get_last_value_wins_semantics() {
    for key in [
        "client_id",
        "id_token_hint",
        "post_logout_redirect_uri",
        "state",
        "ui_locales",
        "logout_hint",
    ] {
        assert!(request(&[(key, "first"), (key, "second")]).is_err());
    }
    assert!(request(&[("state", &"x".repeat(1025))]).is_err());
    assert!(request(&[("state", "line\nbreak")]).is_err());
    assert!(request(&[("ui_locales", &"x".repeat(32_769))]).is_err());
    assert!(request(&[("id_token_hint", "")]).is_err());
    assert!(request(&[("unexpected", "value")]).is_err());
}

#[test]
fn unhinted_logout_never_invents_a_return_target() {
    for fields in [
        vec![],
        vec![("client_id", "account-web")],
        vec![("ui_locales", "fr-CA fr en")],
    ] {
        let request = request(&fields).unwrap();
        assert!(request.hint.is_none());
        assert!(request.return_uri().unwrap().is_none());
    }
}
