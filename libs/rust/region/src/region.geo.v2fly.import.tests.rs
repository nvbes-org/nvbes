use sha2::{Digest, Sha256};
use sqlx::PgPool;

use super::{
    V2FLY_GEOIP_SOURCE_CODE, V2flyGeoIpImportError, decode_sha256_hex, parse_sha256sum,
    relation_key, verify_sha256,
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

fn sample_range() -> V2flyGeoIpRange {
    V2flyGeoIpRange {
        country_code: "FR".into(),
        network: "203.0.113.0/24".parse().unwrap(),
    }
}

async fn import_v2fly_ranges(pool: &PgPool, ranges: &[V2flyGeoIpRange]) -> (u64, u64) {
    use super::{
        create_import_table_tx, ensure_v2fly_geoip_source_tx, expire_missing_ranges_tx,
        insert_import_rows_tx, upsert_imported_ranges_tx,
    };

    let mut tx = pool.begin().await.expect("begin");
    ensure_v2fly_geoip_source_tx(&mut tx).await.expect("source");
    create_import_table_tx(&mut tx).await.expect("temp");
    insert_import_rows_tx(&mut tx, ranges)
        .await
        .expect("insert");
    let imported = upsert_imported_ranges_tx(&mut tx).await.expect("upsert");
    let expired = expire_missing_ranges_tx(&mut tx).await.expect("expire");
    tx.commit().await.expect("commit");
    (imported, expired)
}

#[sqlx::test(migrations = "./migrations")]
async fn upserts_v2fly_geoip_ranges(pool: PgPool) {
    let (imported, _) = import_v2fly_ranges(&pool, &[sample_range()]).await;
    assert_eq!(imported, 1);

    let country: Option<String> = sqlx::query_scalar(
        "SELECT country_code FROM geo_ip_network_relations WHERE source_code = $1",
    )
    .bind(V2FLY_GEOIP_SOURCE_CODE)
    .fetch_one(&pool)
    .await
    .expect("country");
    assert_eq!(country.as_deref(), Some("FR"));
}

#[sqlx::test(migrations = "./migrations")]
async fn scheduled_v2fly_import_skips_when_fresh(pool: PgPool) {
    import_v2fly_ranges(&pool, &[sample_range()]).await;

    let report = super::run_scheduled_v2fly_geoip_import(&pool)
        .await
        .expect("scheduled");
    assert!(!report.downloaded);
    assert_eq!(report.imported_ranges, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn import_latest_downloads_fixture_from_mock_server(pool: PgPool) {
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
        .respond_with(ResponseTemplate::new(200).set_body_bytes(dat))
        .mount(&server)
        .await;

    let sha256_url = format!("{}/geoip.dat.sha256sum", server.uri());
    let dat_url = format!("{}/geoip.dat", server.uri());
    let report = super::import_latest_v2fly_geoip_with_urls(&pool, &sha256_url, &dat_url)
        .await
        .expect("import");
    assert!(report.downloaded);
    assert_eq!(report.imported_ranges, 1);
}
