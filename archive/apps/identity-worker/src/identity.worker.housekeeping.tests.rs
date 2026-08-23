use std::time::{Duration, Instant};

use nvbes_core::config::AppConfig;

use super::{HOUSEKEEPING_INTERVAL, housekeeping_due, maxmind_geolite_config, run_if_due};
use crate::worker::test_support::app_state;

#[test]
fn housekeeping_due_respects_the_hourly_boundary() {
    assert!(!housekeeping_due(&Instant::now()));
    assert!(housekeeping_due(&(Instant::now() - HOUSEKEEPING_INTERVAL)));
    assert!(housekeeping_due(
        &(Instant::now() - Duration::from_secs(7200))
    ));
}

#[test]
fn maxmind_configuration_requires_account_and_license() {
    let mut config = AppConfig::default();
    assert!(
        maxmind_geolite_config(&config)
            .expect_err("account id")
            .to_string()
            .contains("ACCOUNT_ID")
    );
    config.maxmind_account_id = Some("account".to_string());
    assert!(
        maxmind_geolite_config(&config)
            .expect_err("license")
            .to_string()
            .contains("LICENSE_KEY")
    );
}

#[test]
fn maxmind_configuration_preserves_explicit_consent_and_credentials() {
    let config = AppConfig {
        maxmind_account_id: Some("account".to_string()),
        maxmind_license_key: Some("license".to_string()),
        maxmind_geolite_eula_accepted: true,
        ..AppConfig::default()
    };
    let result = maxmind_geolite_config(&config).expect("MaxMind config");
    assert_eq!(result.account_id, "account");
    assert_eq!(result.license_key, "license");
    assert!(result.eula_accepted);
}

#[tokio::test]
async fn run_if_due_is_a_no_op_before_the_hourly_boundary() {
    let Some(state) = app_state().await else {
        return;
    };
    let mut last_run = Instant::now();
    run_if_due(&state, &mut last_run).await.expect("not due");
    assert!(last_run.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn due_housekeeping_runs_database_maintenance_and_all_fresh_importers() {
    let Some(mut state) = app_state().await else {
        return;
    };
    state.config.maxmind_geolite_database_enabled = true;
    state.config.maxmind_account_id = Some("test-account".to_string());
    state.config.maxmind_license_key = Some("test-license".to_string());
    state.config.maxmind_geolite_eula_accepted = true;
    state.config.loyalsoldier_geoip_enabled = true;
    seed_fresh_geo_sources(&state.db).await;
    let mut last_run = Instant::now() - HOUSEKEEPING_INTERVAL;

    run_if_due(&state, &mut last_run)
        .await
        .expect("due housekeeping");

    assert!(last_run.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn enabled_maxmind_import_fails_closed_without_credentials() {
    let Some(mut state) = app_state().await else {
        return;
    };
    state.config.maxmind_geolite_database_enabled = true;
    state.config.maxmind_account_id = None;
    state.config.maxmind_license_key = None;
    let error = super::run_maxmind_geolite_import_if_due(&state)
        .await
        .expect_err("missing MaxMind credentials");
    assert!(error.to_string().contains("MAXMIND_ACCOUNT_ID"));
}

async fn seed_fresh_geo_sources(db: &sqlx::PgPool) {
    for (source, network) in [
        ("v2fly_geoip", "203.0.113.0/24"),
        ("loyalsoldier_geoip", "198.51.100.0/24"),
        ("maxmind_geolite_country_csv", "192.0.2.0/24"),
    ] {
        sqlx::query(
            r#"INSERT INTO geo_ip_network_relations
               (source_code, relation_key, network, country_code, fetched_at, expires_at)
               VALUES ($1, $2, $3::CIDR, 'FR', NOW(), NOW() + INTERVAL '1 day')
               ON CONFLICT (source_code, relation_key) DO UPDATE
               SET fetched_at = NOW(), expires_at = NOW() + INTERVAL '1 day'"#,
        )
        .bind(source)
        .bind(format!("identity-worker-test-{source}"))
        .bind(network)
        .execute(db)
        .await
        .expect("fresh GeoIP fixture");
    }
}
