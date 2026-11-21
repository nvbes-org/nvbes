use super::constant_time_eq;

#[test]
fn constant_time_eq_compares_length_and_content() {
    assert!(constant_time_eq(b"same-token-value", b"same-token-value"));
    assert!(!constant_time_eq(b"same-token-value", b"other-token-value"));
    assert!(!constant_time_eq(b"short", b"much-longer-value"));
    assert!(constant_time_eq(b"", b""));
}
