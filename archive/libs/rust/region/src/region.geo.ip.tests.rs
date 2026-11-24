use super::{is_private_or_special_ip, parse_ip};

#[test]
fn detects_private_and_special_addresses() {
    assert!(is_private_or_special_ip(parse_ip("10.0.0.1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("127.0.0.1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("169.254.1.1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("224.0.0.1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("0.0.0.0").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("0.1.2.3").unwrap()));
    assert!(is_private_or_special_ip(
        parse_ip("255.255.255.255").unwrap()
    ));
    assert!(is_private_or_special_ip(parse_ip("192.0.2.1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("198.51.100.1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("203.0.113.1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("2001:db8::1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("::1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("::").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("fc00::1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("fd12::1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("fe80::1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("ff02::1").unwrap()));
    assert!(is_private_or_special_ip(parse_ip("ff00::1").unwrap()));
    assert!(!is_private_or_special_ip(parse_ip("8.8.8.8").unwrap()));
    assert!(!is_private_or_special_ip(parse_ip("1.1.1.1").unwrap()));
    assert!(!is_private_or_special_ip(
        parse_ip("2001:4860::8888").unwrap()
    ));
}

#[test]
fn parse_ip_trims_and_rejects_garbage() {
    assert!(parse_ip("  8.8.8.8  ").is_some());
    assert!(parse_ip("not-an-ip").is_none());
    assert!(parse_ip("").is_none());
}
