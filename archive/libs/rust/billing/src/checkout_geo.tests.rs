use nvbes_core::config::AppConfig;
use sqlx::PgPool;

use super::resolve_checkout_geo;

const GEO_SCHEMA: &str = r"
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE TABLE IF NOT EXISTS geo_sources (
  code TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  trust_level TEXT NOT NULL,
  priority SMALLINT NOT NULL,
  base_url TEXT,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO geo_sources (code, kind, trust_level, priority)
VALUES
  ('personal_database', 'personal_database', 'high', 20),
  ('remote_lookup', 'rdap', 'medium', 100),
  ('ip_intelligence', 'rdap', 'medium', 80),
  ('maxmind_geolite_city_web', 'rdap', 'medium', 85)
ON CONFLICT (code) DO NOTHING;
CREATE TABLE IF NOT EXISTS geo_ip_network_relations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_code TEXT NOT NULL REFERENCES geo_sources(code),
  relation_key TEXT NOT NULL,
  registry TEXT,
  network CIDR,
  start_ip INET,
  end_ip INET,
  asn BIGINT,
  organization TEXT,
  country_code CHAR(2),
  source_reference TEXT,
  network_kind TEXT NOT NULL DEFAULT 'unknown',
  risk_score SMALLINT NOT NULL DEFAULT 50,
  risk_labels TEXT[] NOT NULL DEFAULT ARRAY['unknown']::TEXT[],
  raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
  fetched_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_geo_ip_network_relations_unique
  ON geo_ip_network_relations (source_code, relation_key);
CREATE TABLE IF NOT EXISTS geo_personal_ip_ranges (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  network CIDR NOT NULL UNIQUE,
  country_code CHAR(2) NOT NULL,
  priority SMALLINT NOT NULL DEFAULT 100,
  source_reference TEXT,
  note TEXT,
  network_kind TEXT NOT NULL DEFAULT 'residential',
  risk_score SMALLINT NOT NULL DEFAULT 15,
  risk_labels TEXT[] NOT NULL DEFAULT ARRAY['personal_database']::TEXT[],
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ
);
";

async fn seed_geo_schema(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) {
    sqlx::raw_sql(GEO_SCHEMA).execute(&mut **tx).await.unwrap();
}

fn test_config() -> AppConfig {
    AppConfig {
        maxmind_geolite_web_enabled: false,
        ip_intelligence_provider_specs: Vec::new(),
        ..AppConfig::default()
    }
}

#[sqlx::test]
async fn resolve_checkout_geo_uses_trusted_country_without_remote_fetch(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let resolution = resolve_checkout_geo(
        &mut tx,
        &test_config(),
        Some("203.0.113.42"),
        Some("DE"),
        None,
    )
    .await
    .expect("geo resolves");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("DE")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_uses_personal_database_for_known_network(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    sqlx::query(
        "INSERT INTO geo_personal_ip_ranges (network, country_code)
         VALUES ('1.2.3.0/24'::cidr, 'FR')",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let resolution = resolve_checkout_geo(&mut tx, &test_config(), Some("1.2.3.4"), None, None)
        .await
        .expect("geo resolves");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("FR")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_reads_cached_remote_lookup(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    sqlx::query(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, network, country_code, network_kind, risk_score, risk_labels
        )
        VALUES ('remote_lookup', 'remote:1.2.4.0/24', '1.2.4.0/24'::cidr, 'NL', 'residential', 20, ARRAY['cached'])
        "#,
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let resolution = resolve_checkout_geo(&mut tx, &test_config(), Some("1.2.4.50"), None, None)
        .await
        .expect("geo resolves");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("NL")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_handles_private_ip_without_network_calls(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let resolution =
        resolve_checkout_geo(&mut tx, &test_config(), Some("10.0.0.4"), None, Some("US"))
            .await
            .expect("geo resolves");
    tx.commit().await.unwrap();

    assert!(resolution.private_network);
    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("US")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_skips_maxmind_when_disabled(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let config = AppConfig {
        maxmind_geolite_web_enabled: true,
        maxmind_account_id: None,
        maxmind_license_key: None,
        ip_intelligence_provider_specs: vec!["broken|not-a-url".to_string()],
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(&mut tx, &config, Some("203.0.113.42"), Some("BE"), None)
        .await
        .expect("geo resolves without external providers");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("BE")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_tolerates_public_ip_when_remote_lookups_fail(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let config = AppConfig {
        maxmind_geolite_web_enabled: false,
        ip_intelligence_provider_specs: Vec::new(),
        maxmind_web_timeout_secs: 1,
        ip_intelligence_timeout_secs: 1,
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(&mut tx, &config, Some("203.0.113.99"), None, Some("FR"))
        .await
        .expect("geo resolves after remote lookup failure");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("FR")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_rejects_invalid_maxmind_configuration(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let config = AppConfig {
        maxmind_geolite_web_enabled: true,
        maxmind_account_id: Some("account".to_string()),
        maxmind_license_key: Some("license".to_string()),
        maxmind_geolite_eula_accepted: false,
        maxmind_web_timeout_secs: 1,
        ip_intelligence_provider_specs: Vec::new(),
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(&mut tx, &config, Some("203.0.113.77"), None, Some("IT"))
        .await
        .expect("invalid maxmind config is ignored");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("IT")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_tolerates_maxmind_network_failure(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let config = AppConfig {
        maxmind_geolite_web_enabled: true,
        maxmind_account_id: Some("account".to_string()),
        maxmind_license_key: Some("license".to_string()),
        maxmind_geolite_eula_accepted: true,
        maxmind_web_timeout_secs: 1,
        ip_intelligence_provider_specs: Vec::new(),
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(&mut tx, &config, Some("203.0.113.88"), None, Some("ES"))
        .await
        .expect("maxmind network failure is ignored");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("ES")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_uses_cached_ip_intelligence(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    sqlx::query(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, network, country_code, network_kind, risk_score, risk_labels
        )
        VALUES (
          'ip_intelligence', 'intel:198.51.100.0/24', '198.51.100.0/24'::cidr, 'CA',
          'hosting', 80, ARRAY['datacenter']
        )
        "#,
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let config = AppConfig {
        maxmind_geolite_web_enabled: false,
        ip_intelligence_provider_specs: vec!["fixture|https://127.0.0.1:1/ip/{ip}".to_string()],
        ip_intelligence_timeout_secs: 1,
        ..AppConfig::default()
    };
    let resolution =
        resolve_checkout_geo(&mut tx, &config, Some("198.51.100.10"), Some("CA"), None)
            .await
            .expect("cached intelligence resolves");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("CA")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_tolerates_ip_intelligence_provider_failure(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let config = AppConfig {
        maxmind_geolite_web_enabled: false,
        ip_intelligence_provider_specs: vec!["fixture|https://127.0.0.1:1/ip/{ip}".to_string()],
        ip_intelligence_timeout_secs: 1,
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(&mut tx, &config, Some("203.0.113.55"), Some("PT"), None)
        .await
        .expect("intelligence provider failure is ignored");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("PT")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_uses_stored_profile_when_ip_absent_or_invalid(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    for ip in [None, Some("not-an-ip"), Some("")] {
        let resolution = resolve_checkout_geo(&mut tx, &test_config(), ip, None, Some("IE"))
            .await
            .expect("geo resolves without ip");
        assert_eq!(
            resolution
                .location
                .as_ref()
                .map(|location| location.country_code.as_str()),
            Some("IE")
        );
    }
    tx.commit().await.unwrap();
}

#[sqlx::test]
async fn resolve_checkout_geo_skips_maxmind_when_credentials_incomplete(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;

    let account_only = AppConfig {
        maxmind_geolite_web_enabled: true,
        maxmind_account_id: Some("account".to_string()),
        maxmind_license_key: None,
        maxmind_geolite_eula_accepted: true,
        ip_intelligence_provider_specs: Vec::new(),
        maxmind_web_timeout_secs: 1,
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(
        &mut tx,
        &account_only,
        Some("203.0.113.41"),
        None,
        Some("AT"),
    )
    .await
    .expect("missing license skips maxmind");
    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("AT")
    );

    let license_only = AppConfig {
        maxmind_geolite_web_enabled: true,
        maxmind_account_id: None,
        maxmind_license_key: Some("license".to_string()),
        maxmind_geolite_eula_accepted: true,
        ip_intelligence_provider_specs: Vec::new(),
        maxmind_web_timeout_secs: 1,
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(
        &mut tx,
        &license_only,
        Some("203.0.113.42"),
        None,
        Some("CH"),
    )
    .await
    .expect("missing account skips maxmind");
    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("CH")
    );
    tx.commit().await.unwrap();
}

#[sqlx::test]
async fn resolve_checkout_geo_ignores_invalid_ip_intelligence_specs_without_trusted_header(
    pool: PgPool,
) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let config = AppConfig {
        maxmind_geolite_web_enabled: false,
        ip_intelligence_provider_specs: vec!["broken|not-a-url".to_string()],
        ip_intelligence_timeout_secs: 1,
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(&mut tx, &config, Some("203.0.113.66"), None, Some("NO"))
        .await
        .expect("invalid intelligence specs are ignored");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("NO")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_fetches_intelligence_for_public_ip_with_trusted_header(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let config = AppConfig {
        maxmind_geolite_web_enabled: false,
        ip_intelligence_provider_specs: vec!["fixture|https://127.0.0.1:1/ip/{ip}".to_string()],
        ip_intelligence_timeout_secs: 1,
        ..AppConfig::default()
    };
    // 8.8.8.8 is a public address (not documentation/special), so intelligence is attempted.
    let resolution = resolve_checkout_geo(&mut tx, &config, Some("8.8.8.8"), Some("US"), None)
        .await
        .expect("public ip intelligence failure is ignored");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("US")
    );
}

#[sqlx::test]
async fn resolve_checkout_geo_attempts_maxmind_for_public_ip_when_enabled(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    seed_geo_schema(&mut tx).await;
    let config = AppConfig {
        maxmind_geolite_web_enabled: true,
        maxmind_account_id: Some("account".to_string()),
        maxmind_license_key: Some("license".to_string()),
        maxmind_geolite_eula_accepted: true,
        maxmind_web_timeout_secs: 1,
        ip_intelligence_provider_specs: Vec::new(),
        ..AppConfig::default()
    };
    let resolution = resolve_checkout_geo(&mut tx, &config, Some("1.1.1.1"), None, Some("AU"))
        .await
        .expect("maxmind network failure is ignored for public ip");
    tx.commit().await.unwrap();

    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("AU")
    );
}
