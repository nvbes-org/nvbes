use super::{
    generate_random_token, generate_token, random_challenge, token_hash, token_hash_b64,
    unique_slug,
};

#[test]
fn generate_token_includes_prefix() {
    let token = generate_token("verify");
    assert!(token.starts_with("verify_"));
    assert!(token.len() > "verify_".len());
}

#[test]
fn generate_random_token_is_url_safe_and_unique() {
    let first = generate_random_token();
    let second = generate_random_token();
    assert_ne!(first, second);
    assert!(
        first
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    );
}

#[test]
fn random_challenge_returns_32_bytes() {
    assert_eq!(random_challenge().len(), 32);
}

#[test]
fn token_hash_is_stable_hex_digest() {
    let digest = token_hash("example-token");
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_eq!(digest, token_hash("example-token"));
}

#[test]
fn token_hash_b64_differs_from_hex_form() {
    let hex = token_hash("example-token");
    let b64 = token_hash_b64("example-token");
    assert_ne!(hex, b64);
    assert!(!b64.contains('+'));
}

#[test]
fn unique_slug_uses_email_local_part_and_suffix() {
    let slug = unique_slug("ada@example.com");
    assert!(slug.starts_with("ada-"));
    assert_eq!(slug.len(), "ada-".len() + 8);
}

#[test]
fn unique_slug_falls_back_to_tenant_for_empty_seed() {
    let slug = unique_slug("@@@");
    assert!(slug.starts_with("tenant-"));
}
