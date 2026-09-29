use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn generate_token(prefix: &str) -> String {
    format!("{prefix}_{}", generate_random_token())
}

pub fn generate_random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn random_challenge() -> Vec<u8> {
    let mut bytes = vec![0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

pub fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn token_hash_b64(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn unique_slug(seed: &str) -> String {
    let base = seed
        .split('@')
        .next()
        .unwrap_or(seed)
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let suffix = Uuid::new_v4().simple().to_string();
    format!(
        "{}-{}",
        if base.is_empty() { "tenant" } else { &base },
        &suffix[..8]
    )
}

#[cfg(test)]
#[path = "auth.helpers.tokens.tests.rs"]
mod tests;
