use chrono::{Duration, Utc};
use sqlx::PgPool;

use super::{
    UpsertPersonalGeoRangeInput, disable_personal_geo_range, list_personal_geo_ranges,
    upsert_personal_geo_range,
};
use crate::geo::types::GeoNetworkKind;

fn valid_input() -> UpsertPersonalGeoRangeInput {
    UpsertPersonalGeoRangeInput {
        network: "203.0.113.0/24".parse().unwrap(),
        country_code: "FR".to_string(),
        priority: 100,
        source_reference: Some("manual-test".to_string()),
        note: None,
        network_kind: GeoNetworkKind::Datacenter,
        risk_score: 70,
        risk_labels: vec!["manual".to_string(), "datacenter".to_string()],
        expires_at: None,
    }
}

#[test]
fn validates_country_code_and_priority_before_write() {
    use super::{PersonalGeoStoreError, validate_personal_geo_input};

    assert!(validate_personal_geo_input(&valid_input()).is_ok());

    let mut invalid_country = valid_input();
    invalid_country.country_code = "XX".to_string();
    assert!(matches!(
        validate_personal_geo_input(&invalid_country),
        Err(PersonalGeoStoreError::Validation(_))
    ));

    let mut invalid_priority = valid_input();
    invalid_priority.priority = 0;
    assert!(matches!(
        validate_personal_geo_input(&invalid_priority),
        Err(PersonalGeoStoreError::InvalidPriority)
    ));
}

#[test]
fn rejects_risk_scores_above_one_hundred() {
    use super::{PersonalGeoStoreError, validate_personal_geo_input};

    let mut input = valid_input();
    input.risk_score = 101;
    assert!(matches!(
        validate_personal_geo_input(&input),
        Err(PersonalGeoStoreError::InvalidRiskScore)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn upserts_lists_and_disables_personal_ranges(pool: PgPool) {
    let view = upsert_personal_geo_range(&pool, valid_input())
        .await
        .expect("upsert");
    assert_eq!(view.country_code, "FR");
    assert!(view.enabled);

    let listed = list_personal_geo_ranges(&pool).await.expect("list");
    assert_eq!(listed.len(), 1);

    let disabled = disable_personal_geo_range(&pool, "203.0.113.0/24".parse().unwrap())
        .await
        .expect("disable")
        .expect("row");
    assert!(!disabled.enabled);
}

#[sqlx::test(migrations = "./migrations")]
async fn upsert_updates_existing_network(pool: PgPool) {
    upsert_personal_geo_range(&pool, valid_input())
        .await
        .expect("insert");

    let mut updated = valid_input();
    updated.country_code = "DE".to_string();
    updated.expires_at = Some(Utc::now() + Duration::days(30));
    let view = upsert_personal_geo_range(&pool, updated)
        .await
        .expect("update");
    assert_eq!(view.country_code, "DE");
}
