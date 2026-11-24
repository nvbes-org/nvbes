use sqlx::PgPool;

use super::relation_key;
use crate::geo::maxmind_types::{
    MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE, MaxMindGeoLiteConfig, MaxMindGeoLiteRange,
};

#[test]
fn relation_key_includes_source_and_network() {
    let range = MaxMindGeoLiteRange {
        source_code: "maxmind_geolite_country_csv",
        edition_id: "GeoLite2-Country-CSV",
        network: "203.0.113.0/24".parse().unwrap(),
        country_code: Some("FR".into()),
        geoname_id: None,
        asn: None,
        organization: None,
    };
    assert_eq!(
        relation_key(&range),
        "maxmind_geolite_country_csv:203.0.113.0/24"
    );
}

fn sample_country_range() -> MaxMindGeoLiteRange {
    MaxMindGeoLiteRange {
        source_code: MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE,
        edition_id: "GeoLite2-Country-CSV",
        network: "203.0.113.0/24".parse().unwrap(),
        country_code: Some("FR".into()),
        geoname_id: Some("3017382".into()),
        asn: None,
        organization: None,
    }
}

async fn import_ranges(pool: &PgPool, ranges: &[MaxMindGeoLiteRange]) -> (u64, u64) {
    use super::{
        create_import_table_tx, ensure_geolite_source_tx, expire_missing_ranges_tx,
        insert_import_rows_tx, upsert_imported_ranges_tx,
    };

    let mut tx = pool.begin().await.expect("begin");
    ensure_geolite_source_tx(&mut tx).await.expect("sources");
    create_import_table_tx(&mut tx).await.expect("temp table");
    insert_import_rows_tx(&mut tx, ranges)
        .await
        .expect("insert temp");
    let imported = upsert_imported_ranges_tx(&mut tx).await.expect("upsert");
    let expired = expire_missing_ranges_tx(&mut tx).await.expect("expire");
    tx.commit().await.expect("commit");
    (imported, expired)
}

#[sqlx::test(migrations = "./migrations")]
async fn upserts_maxmind_geolite_ranges(pool: PgPool) {
    let (imported, expired) = import_ranges(&pool, &[sample_country_range()]).await;
    assert_eq!(imported, 1);
    assert_eq!(expired, 0);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM geo_ip_network_relations WHERE source_code = $1")
            .bind(MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn expires_missing_maxmind_relations(pool: PgPool) {
    import_ranges(&pool, &[sample_country_range()]).await;

    let stale = MaxMindGeoLiteRange {
        source_code: MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE,
        edition_id: "GeoLite2-Country-CSV",
        network: "198.51.100.0/24".parse().unwrap(),
        country_code: Some("US".into()),
        geoname_id: None,
        asn: None,
        organization: None,
    };
    import_ranges(&pool, &[stale]).await;

    let replacement = MaxMindGeoLiteRange {
        network: "203.0.113.0/24".parse().unwrap(),
        ..sample_country_range()
    };
    let (_, expired) = import_ranges(&pool, &[replacement]).await;
    assert_eq!(expired, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn scheduled_import_skips_when_data_is_fresh(pool: PgPool) {
    import_ranges(&pool, &[sample_country_range()]).await;

    let report = super::run_scheduled_maxmind_geolite_import(
        &pool,
        MaxMindGeoLiteConfig {
            account_id: "test-account".to_string(),
            license_key: "test-key".to_string(),
            eula_accepted: true,
        },
    )
    .await
    .expect("scheduled import");

    assert!(!report.downloaded);
    assert_eq!(report.imported_ranges, 0);
}
