use sqlx::PgPool;
use uuid::Uuid;

use super::run_geo_maintenance_tx;
use crate::geo::maxmind_types::MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE;

#[sqlx::test(migrations = "./migrations")]
async fn disables_expired_personal_ranges(pool: PgPool) {
    sqlx::query(
        r#"
        INSERT INTO geo_personal_ip_ranges (
          network, country_code, priority, enabled, expires_at
        )
        VALUES ('203.0.113.0/24', 'FR', 10, TRUE, now() - interval '1 hour')
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert personal");

    let mut tx = pool.begin().await.expect("begin");
    let report = run_geo_maintenance_tx(&mut tx).await.expect("maintenance");
    tx.commit().await.expect("commit");

    assert_eq!(report.expired_personal_ranges_disabled, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn scrubs_and_deletes_expired_maxmind_relations(pool: PgPool) {
    let relation_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, country_code,
          network_kind, risk_score, risk_labels, expires_at
        )
        VALUES (
          $1, 'fixture:203.0.113.0/24', 'maxmind', '203.0.113.0/24', 'FR',
          'unknown', 30, ARRAY['source:maxmind_geolite'], now() - interval '1 hour'
        )
        RETURNING id
        "#,
    )
    .bind(MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE)
    .fetch_one(&pool)
    .await
    .expect("insert relation");

    let event_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO geo_lookup_events (
          purpose, selected_source, confidence, network_kind, risk_score, risk_labels
        )
        VALUES ('audit', 'fallback', 'medium', 'unknown', 50, ARRAY['unknown'])
        RETURNING id
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("event");

    sqlx::query(
        r#"
        INSERT INTO geo_lookup_evidence (
          lookup_event_id, source_code, network_relation_id, confidence, accepted, reason
        )
        VALUES ($1, 'ripe', $2, 'medium', TRUE, 'fixture')
        "#,
    )
    .bind(event_id)
    .bind(relation_id)
    .execute(&pool)
    .await
    .expect("evidence");

    let mut tx = pool.begin().await.expect("begin");
    let report = run_geo_maintenance_tx(&mut tx).await.expect("maintenance");
    tx.commit().await.expect("commit");

    assert_eq!(report.expired_maxmind_evidence_scrubbed, 1);
    assert_eq!(report.expired_maxmind_relations_deleted, 1);

    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM geo_ip_network_relations WHERE id = $1")
            .bind(relation_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(remaining, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn run_geo_maintenance_public_entrypoint(pool: PgPool) {
    let report = super::run_geo_maintenance(&pool)
        .await
        .expect("maintenance");
    assert_eq!(report.expired_personal_ranges_disabled, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn deletes_unreferenced_expired_relations(pool: PgPool) {
    sqlx::query(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, country_code,
          network_kind, risk_score, risk_labels, expires_at
        )
        VALUES (
          'v2fly_geoip', 'US:198.51.100.0/24', 'v2fly', '198.51.100.0/24', 'US',
          'unknown', 35, ARRAY['source:v2fly_geoip'], now() - interval '1 day'
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert stale relation");

    let mut tx = pool.begin().await.expect("begin");
    let report = run_geo_maintenance_tx(&mut tx).await.expect("maintenance");
    tx.commit().await.expect("commit");

    assert_eq!(report.expired_unreferenced_relations_deleted, 1);
}
