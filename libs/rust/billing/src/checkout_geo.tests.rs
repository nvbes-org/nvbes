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
