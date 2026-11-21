use base64::Engine;
use sha2::{Digest, Sha256};

use super::validate_pkce;

#[test]
fn validate_pkce_plain_requires_exact_match() {
    assert!(validate_pkce("verifier", "plain", "verifier"));
    assert!(!validate_pkce("challenge", "plain", "verifier"));
}

#[test]
fn validate_pkce_s256_matches_url_safe_sha256() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let hash = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash);
    assert!(validate_pkce(&challenge, "S256", verifier));
    assert!(!validate_pkce(&challenge, "S256", "wrong-verifier"));
}

#[test]
fn validate_pkce_rejects_unknown_methods() {
    assert!(!validate_pkce("any", "S512", "any"));
    assert!(!validate_pkce("any", "", "any"));
}
