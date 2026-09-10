use super::{hash_token, normalize_email, random_token};

#[test]
fn email_is_normalized_without_leaking_into_a_token() {
    assert_eq!(
        normalize_email(" Person@Example.COM ").unwrap(),
        "person@example.com"
    );
    assert!(normalize_email("invalid").is_err());
}

#[test]
fn opaque_tokens_have_fixed_entropy_and_are_hashed_at_rest() {
    let first = random_token();
    let second = random_token();
    assert_eq!(first.len(), 43);
    assert_ne!(first, second);
    assert_eq!(hash_token(&first).len(), 32);
    assert_ne!(hash_token(&first), first.as_bytes());
}
