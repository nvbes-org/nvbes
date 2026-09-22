use sqlx::PgPool;

use super::{
    cached_ip_intelligence_tx, cached_remote_lookup_tx, merge_cached_intelligence,
    resolve_cached_geo_tx,
};
use crate::geo::intelligence::IpIntelligenceInput;
use crate::geo::intelligence::{IpIntelligenceLookup, normalize_ip_intelligence};
use crate::geo::types::{GeoNetworkKind, GeoNetworkRelation};

#[test]
fn merge_cached_intelligence_returns_original_without_lookup() {
    let relation = GeoNetworkRelation {
        source_code: "ripe".to_string(),
        registry: Some("ripe".to_string()),
        network: Some("203.0.113.0/24".to_string()),
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: Some(GeoNetworkKind::Unknown),
        risk_score: Some(30),
        risk_labels: vec!["source:ripe".to_string()],
    };
    let merged = merge_cached_intelligence(relation.clone(), None);
    assert_eq!(merged, relation);
}

#[test]
fn merge_cached_intelligence_prefers_higher_risk_intelligence() {
    let relation = GeoNetworkRelation {
        source_code: "ripe".to_string(),
        registry: Some("ripe".to_string()),
        network: Some("203.0.113.0/24".to_string()),
        start_ip: None,
        end_ip: None,
        asn: None,
        organization: None,
        source_reference: None,
        network_kind: Some(GeoNetworkKind::Unknown),
        risk_score: Some(30),
        risk_labels: vec!["source:ripe".to_string()],
    };
    let intelligence = IpIntelligenceLookup {
        country_code: Some("FR".to_string()),
        relation: normalize_ip_intelligence(IpIntelligenceInput {
            source_code: "test_provider".to_string(),
            ip: "203.0.113.42".parse().unwrap(),
            country_code: Some("FR".to_string()),
            asn: None,
            organization: None,
            network: Some("203.0.113.0/24".to_string()),
            source_reference: None,
            is_vpn: true,
            is_proxy: false,
            is_tor: false,
            is_datacenter: false,
            is_mobile: false,
            is_residential: false,
            risk_score: None,
            risk_labels: Vec::new(),
        })
        .relation,
    };
    let merged = merge_cached_intelligence(relation, Some(&intelligence));
    assert_eq!(merged.network_kind, Some(GeoNetworkKind::Vpn));
    assert_eq!(merged.risk_score, Some(90));
}

async fn seed_cached_relation(pool: &PgPool) {
    sqlx::query(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, country_code,
          network_kind, risk_score, risk_labels
        )
        VALUES (
          'ripe', 'network:93.184.216.0/24', 'ripe', '93.184.216.0/24', 'US',
          'residential', 15, ARRAY['source:ripe']
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("seed relation");
}

#[sqlx::test(migrations = "./migrations")]
async fn cached_remote_lookup_returns_rdap_lookup(pool: PgPool) {
    seed_cached_relation(&pool).await;
    let mut tx = pool.begin().await.expect("begin");
    let lookup = cached_remote_lookup_tx(&mut tx, "93.184.216.34".parse().unwrap())
        .await
        .expect("lookup")
        .expect("hit");
    tx.commit().await.expect("commit");
    assert_eq!(lookup.location.country_code, "US");
}

#[sqlx::test(migrations = "./migrations")]
async fn resolve_cached_geo_uses_personal_and_cached_relations(pool: PgPool) {
    seed_cached_relation(&pool).await;
    sqlx::query(
        r#"
        INSERT INTO geo_personal_ip_ranges (
          network, country_code, priority, network_kind, risk_score, risk_labels
        )
        VALUES ('198.51.100.0/24', 'US', 5, 'residential', 15, ARRAY['personal_database'])
        "#,
    )
    .execute(&pool)
    .await
    .expect("personal");

    let mut tx = pool.begin().await.expect("begin");
    let resolution = resolve_cached_geo_tx(&mut tx, Some("93.184.216.34"), None, None)
        .await
        .expect("resolve");
    tx.commit().await.expect("commit");
    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|l| l.country_code.as_str()),
        Some("US")
    );

    let mut tx = pool.begin().await.expect("begin");
    let intelligence = cached_ip_intelligence_tx(&mut tx, "93.184.216.34".parse().unwrap())
        .await
        .expect("intel");
    tx.commit().await.expect("commit");
    assert!(intelligence.is_some());
}
