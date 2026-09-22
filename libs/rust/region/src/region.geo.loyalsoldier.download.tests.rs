use sha2::{Digest, Sha256};

use super::{LoyalsoldierGeoIpDownloadError, decode_sha256_hex, parse_sha256sum, verify_sha256};

#[test]
fn parses_sha256sum_file() {
    let checksum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  geoip.dat";
    assert!(parse_sha256sum(checksum).is_ok());
}

#[test]
fn rejects_invalid_sha256sum() {
    assert!(decode_sha256_hex("abc").is_none());
    assert!(decode_sha256_hex(&"g".repeat(64)).is_none());
    assert!(parse_sha256sum("").is_err());
    assert!(matches!(
        parse_sha256sum("xyz").unwrap_err(),
        LoyalsoldierGeoIpDownloadError::InvalidChecksum
    ));
}

#[test]
fn verify_sha256_accepts_matching_digest() {
    let payload = b"local-loyalsoldier-fixture";
    let digest = Sha256::digest(payload);
    let mut expected = [0_u8; 32];
    expected.copy_from_slice(&digest);
    assert!(verify_sha256(payload, expected).is_ok());
    assert!(matches!(
        verify_sha256(b"other", expected).unwrap_err(),
        LoyalsoldierGeoIpDownloadError::ChecksumMismatch
    ));
}
