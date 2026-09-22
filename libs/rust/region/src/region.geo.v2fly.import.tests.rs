use sha2::{Digest, Sha256};

use super::{
    V2flyGeoIpImportError, decode_sha256_hex, parse_sha256sum, relation_key, verify_sha256,
};
use crate::geo::v2fly_dat::V2flyGeoIpRange;

#[test]
fn parses_sha256sum_file() {
    let checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  geoip.dat";
    assert!(parse_sha256sum(checksum).is_ok());
}

#[test]
fn rejects_invalid_sha256sum() {
    assert!(decode_sha256_hex("abc").is_none());
    assert!(parse_sha256sum("").is_err());
}

#[test]
fn verify_sha256_matches_local_bytes() {
    let payload = b"v2fly-local-fixture";
    let digest = Sha256::digest(payload);
    let mut expected = [0_u8; 32];
    expected.copy_from_slice(&digest);
    assert!(verify_sha256(payload, expected).is_ok());
    assert!(matches!(
        verify_sha256(b"nope", expected).unwrap_err(),
        V2flyGeoIpImportError::ChecksumMismatch
    ));
}

#[test]
fn relation_key_includes_country_and_network() {
    let range = V2flyGeoIpRange {
        country_code: "FR".into(),
        network: "203.0.113.0/24".parse().unwrap(),
    };
    assert_eq!(relation_key(&range), "FR:203.0.113.0/24");
}
