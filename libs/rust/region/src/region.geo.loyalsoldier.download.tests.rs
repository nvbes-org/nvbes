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

#[tokio::test]
async fn download_verified_geoip_reads_local_fixtures_from_mock_server() {
    use reqwest::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::geo::v2fly_dat::test_encode_country_dat;

    let dat = test_encode_country_dat("us", "93.184.216.0/24".parse().unwrap());
    let digest = Sha256::digest(&dat);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let checksum = format!("{hex}  geoip.dat");

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/geoip.dat.sha256sum"))
        .respond_with(ResponseTemplate::new(200).set_body_string(checksum))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/geoip.dat"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(dat.clone()))
        .mount(&server)
        .await;

    let downloaded = super::download_verified_loyalsoldier_geoip_with_urls(
        &Client::new(),
        &format!("{}/geoip.dat.sha256sum", server.uri()),
        &format!("{}/geoip.dat", server.uri()),
    )
    .await
    .expect("download");
    assert_eq!(downloaded, dat);
}
