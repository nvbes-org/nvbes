use sqlx::PgPool;

use crate::geo::loyalsoldier_types::LOYALSOLDIER_GEOIP_SOURCE_CODE;
use crate::geo::v2fly_dat::V2flyGeoIpDatEntry;

fn sample_entries() -> Vec<V2flyGeoIpDatEntry> {
    vec![
        V2flyGeoIpDatEntry {
            code: "fr".to_string(),
            network: "203.0.113.0/24".parse().unwrap(),
        },
        V2flyGeoIpDatEntry {
            code: "tor".to_string(),
            network: "198.51.100.0/24".parse().unwrap(),
        },
    ]
}

async fn import_loyalsoldier_entries(pool: &PgPool, entries: &[V2flyGeoIpDatEntry]) -> (u64, u64) {
    use super::{
        create_import_table_tx, ensure_loyalsoldier_geoip_source_tx, expire_missing_ranges_tx,
        insert_import_rows_tx, upsert_imported_ranges_tx,
    };

    let mut tx = pool.begin().await.expect("begin");
    ensure_loyalsoldier_geoip_source_tx(&mut tx)
        .await
        .expect("source");
    create_import_table_tx(&mut tx).await.expect("temp");
    insert_import_rows_tx(&mut tx, entries)
        .await
        .expect("insert");
    let imported = upsert_imported_ranges_tx(&mut tx).await.expect("upsert");
    let expired = expire_missing_ranges_tx(&mut tx).await.expect("expire");
    tx.commit().await.expect("commit");
    (imported, expired)
}

#[sqlx::test(migrations = "./migrations")]
async fn upserts_loyalsoldier_country_and_category_ranges(pool: PgPool) {
    let entries = sample_entries();
    let (imported, _) = import_loyalsoldier_entries(&pool, &entries).await;
    assert_eq!(imported, 2);

    let tor_kind: String = sqlx::query_scalar(
        r#"
        SELECT network_kind
        FROM geo_ip_network_relations
        WHERE source_code = $1 AND relation_key LIKE 'tor:%'
        "#,
    )
    .bind(LOYALSOLDIER_GEOIP_SOURCE_CODE)
    .fetch_one(&pool)
    .await
    .expect("tor row");
    assert_eq!(tor_kind, "tor");
}

#[sqlx::test(migrations = "./migrations")]
async fn scheduled_loyalsoldier_import_skips_when_fresh(pool: PgPool) {
    import_loyalsoldier_entries(&pool, &sample_entries()).await;

    let report = super::run_scheduled_loyalsoldier_geoip_import(&pool)
        .await
        .expect("scheduled");
    assert!(!report.downloaded);
    assert_eq!(report.imported_ranges, 0);
}
