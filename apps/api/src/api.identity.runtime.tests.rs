use super::*;
use std::{collections::HashMap, sync::OnceLock};

use base64::{Engine, engine::general_purpose::STANDARD};

/// One generated RSA pair shared by every production fixture.
fn signing_keys() -> &'static (String, String) {
    static KEYS: OnceLock<(String, String)> = OnceLock::new();
    KEYS.get_or_init(|| {
        let private = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).unwrap();
        (
            rsa::pkcs8::EncodePrivateKey::to_pkcs8_pem(&private, Default::default())
                .unwrap()
                .to_string(),
            rsa::pkcs8::EncodePublicKey::to_public_key_pem(
                &private.to_public_key(),
                Default::default(),
            )
            .unwrap(),
        )
    })
}

fn production(overrides: &[(&str, Option<&str>)]) -> Result<IdentityRuntime, ConfigError> {
    let (private_pem, public_pem) = signing_keys();
    let secret = STANDARD.encode([7u8; 32]);
    let clients = DEV_CLIENTS_JSON
        .replace("http://localhost:3001", "https://account.nvbes.fr")
        .replace("http://localhost:8080", "https://api.nvbes.fr");
    let mut values: HashMap<&str, String> = HashMap::from([
        (
            "NVBES_IDENTITY_TOKEN_ISSUER",
            "https://api.nvbes.fr".to_owned(),
        ),
        ("NVBES_IDENTITY_TOKEN_KEY_ID", "identity-2026-11".to_owned()),
        ("NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM", private_pem.clone()),
        ("NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM", public_pem.clone()),
        (
            "NVBES_IDENTITY_TOKEN_AUDIENCES",
            "nvbes-account-service".to_owned(),
        ),
        ("NVBES_IDENTITY_OAUTH_CLIENTS_JSON", clients),
        ("NVBES_IDENTITY_RATE_LIMIT_KEY", secret.clone()),
        ("NVBES_IDENTITY_MFA_ENCRYPTION_KEY", secret),
        (
            "NVBES_IDENTITY_BROWSER_ORIGIN",
            "https://id.nvbes.fr".to_owned(),
        ),
    ]);
    for (name, value) in overrides {
        match value {
            Some(value) => values.insert(name, (*value).to_owned()),
            None => values.remove(name),
        };
    }
    IdentityRuntime::from_lookup("production", &|name| values.get(name).cloned())
}

#[test]
fn complete_production_identity_loads_and_shares_its_trust() {
    let runtime = production(&[]).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(runtime.trust.issuer, "https://api.nvbes.fr");
    assert_eq!(runtime.trust.key_id, "identity-2026-11");
    assert_eq!(runtime.trust.verification_keys, "[]");
    assert!(runtime.resources.is_none());
}

#[test]
fn production_never_generates_signing_keys() {
    assert!(matches!(
        production(&[("NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM", None)]),
        Err(ConfigError::Identity(_))
    ));
}

#[test]
fn production_never_generates_an_mfa_key() {
    assert!(matches!(
        production(&[("NVBES_IDENTITY_MFA_ENCRYPTION_KEY", None)]),
        Err(ConfigError::Missing("NVBES_IDENTITY_MFA_ENCRYPTION_KEY"))
    ));
}

#[test]
fn production_never_generates_a_rate_limit_key() {
    assert!(matches!(
        production(&[("NVBES_IDENTITY_RATE_LIMIT_KEY", None)]),
        Err(ConfigError::Missing("NVBES_IDENTITY_RATE_LIMIT_KEY"))
    ));
    assert!(matches!(
        production(&[("NVBES_IDENTITY_RATE_LIMIT_KEY", Some("short"))]),
        Err(ConfigError::Invalid("NVBES_IDENTITY_RATE_LIMIT_KEY"))
    ));
}

#[test]
fn production_requires_registered_clients_and_browser_origin() {
    assert!(matches!(
        production(&[("NVBES_IDENTITY_OAUTH_CLIENTS_JSON", None)]),
        Err(ConfigError::Missing("NVBES_IDENTITY_OAUTH_CLIENTS_JSON"))
    ));
    assert!(matches!(
        production(&[("NVBES_IDENTITY_BROWSER_ORIGIN", None)]),
        Err(ConfigError::Missing("NVBES_IDENTITY_BROWSER_ORIGIN"))
    ));
}

#[test]
fn invalid_resource_servers_are_rejected_instead_of_ignored() {
    assert!(matches!(
        production(&[("NVBES_IDENTITY_RESOURCE_SERVERS_JSON", Some("{"))]),
        Err(ConfigError::Invalid("NVBES_IDENTITY_RESOURCE_SERVERS_JSON"))
    ));
}

#[test]
fn development_runs_without_any_identity_variable() {
    let runtime = IdentityRuntime::from_lookup("development", &|_| None).expect("development");
    assert_eq!(runtime.trust.issuer, DEV_ISSUER);
    assert_eq!(runtime.tokens.issuer(), DEV_ISSUER);
}

#[test]
fn development_still_rejects_a_partial_signing_configuration() {
    let (private_pem, _) = signing_keys();
    let private_pem = private_pem.clone();
    assert!(matches!(
        IdentityRuntime::from_lookup("development", &|name| {
            (name == "NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM").then(|| private_pem.clone())
        }),
        Err(ConfigError::Identity(_))
    ));
}
