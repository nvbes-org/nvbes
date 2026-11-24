use sqlx::PgPool;
use uuid::Uuid;

use super::{
    GeoLookupPurpose, GeoLookupRecordContext, load_personal_geo_database_tx,
    record_geo_resolution_tx,
};
use crate::geo::types::{
    GeoConfidence, GeoEvidence, GeoEvidenceSource, GeoLocation, GeoNetworkKind, GeoNetworkRelation,
    GeoResolution,
};

#[test]
fn geo_lookup_purpose_matches_database_labels() {
    let purposes = [
        (GeoLookupPurpose::Payment, "payment"),
        (GeoLookupPurpose::Security, "security"),
        (GeoLookupPurpose::DataRegion, "data_region"),
        (GeoLookupPurpose::Auth, "auth"),
        (GeoLookupPurpose::Audit, "audit"),
        (GeoLookupPurpose::DriveApi, "drive_api"),
        (GeoLookupPurpose::DriveAudit, "drive_audit"),
    ];

    for (purpose, database_label) in purposes {
        assert_eq!(purpose.as_str(), database_label);
    }
}

fn sample_resolution() -> GeoResolution {
    let location = GeoLocation::from_country_code("FR").expect("FR");
    GeoResolution {
        location: Some(location.clone()),
        confidence: GeoConfidence::Medium,
        source: GeoEvidenceSource::RemoteLookup,
        ip: Some("203.0.113.42".parse().unwrap()),
        private_network: false,
        network_kind: GeoNetworkKind::Residential,
        risk_score: 15,
        risk_labels: vec!["residential".to_string()],
        evidence: vec![GeoEvidence {
            source: GeoEvidenceSource::RemoteLookup,
            country_code: Some(location.country_code.clone()),
            confidence: GeoConfidence::Medium,
            accepted: true,
            reason: "fixture".to_string(),
            relation: Some(GeoNetworkRelation {
                source_code: "ripe".to_string(),
                registry: Some("ripe".to_string()),
                network: Some("203.0.113.0/24".to_string()),
                start_ip: None,
                end_ip: None,
                asn: Some(64500),
                organization: Some("Example".to_string()),
                source_reference: Some("NET-EXAMPLE".to_string()),
                network_kind: Some(GeoNetworkKind::Residential),
                risk_score: Some(15),
                risk_labels: vec!["source:ripe".to_string()],
            }),
        }],
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn records_geo_resolution_and_evidence(pool: PgPool) {
    let mut tx = pool.begin().await.expect("begin");
    let event_id = record_geo_resolution_tx(
        &mut tx,
        GeoLookupRecordContext {
            purpose: GeoLookupPurpose::Security,
            subject_type: Some("user"),
            subject_id: Some(Uuid::new_v4()),
            request_id: Some("req-fixture"),
        },
        &sample_resolution(),
    )
    .await
    .expect("record");
    tx.commit().await.expect("commit");

    let evidence_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM geo_lookup_evidence WHERE lookup_event_id = $1")
            .bind(event_id)
            .fetch_one(&pool)
            .await
            .expect("evidence count");
    assert_eq!(evidence_count, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn loads_enabled_personal_geo_ranges(pool: PgPool) {
    sqlx::query(
        r#"
        INSERT INTO geo_personal_ip_ranges (
          network, country_code, priority, network_kind, risk_score, risk_labels
        )
        VALUES ('203.0.113.0/24', 'FR', 10, 'datacenter', 70, ARRAY['manual'])
        "#,
    )
    .execute(&pool)
    .await
    .expect("insert personal range");

    let mut tx = pool.begin().await.expect("begin");
    let database = load_personal_geo_database_tx(&mut tx).await.expect("load");
    tx.commit().await.expect("commit");

    let location = database
        .lookup("203.0.113.10".parse().unwrap())
        .expect("lookup");
    assert_eq!(location.country_code, "FR");
}
