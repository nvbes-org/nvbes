INSERT INTO geo_sources (code, kind, trust_level, priority, base_url)
VALUES
  (
    'maxmind_geolite_country_csv',
    'rdap',
    'medium',
    82,
    'https://www.maxmind.com/en/geolite-free-ip-geolocation-data'
  ),
  (
    'maxmind_geolite_city_csv',
    'rdap',
    'medium',
    81,
    'https://www.maxmind.com/en/geolite-free-ip-geolocation-data'
  ),
  (
    'maxmind_geolite_asn_csv',
    'rdap',
    'medium',
    84,
    'https://www.maxmind.com/en/geolite-free-ip-geolocation-data'
  ),
  (
    'maxmind_geolite_city_web',
    'rdap',
    'medium',
    83,
    'https://geolite.info/geoip/v2.1/city'
  )
ON CONFLICT (code) DO UPDATE SET
  kind = EXCLUDED.kind,
  trust_level = EXCLUDED.trust_level,
  priority = EXCLUDED.priority,
  base_url = EXCLUDED.base_url,
  enabled = TRUE,
  updated_at = now();

CREATE INDEX IF NOT EXISTS idx_geo_ip_network_relations_maxmind_freshness
  ON geo_ip_network_relations (source_code, fetched_at DESC, expires_at)
  WHERE source_code IN (
    'maxmind_geolite_country_csv',
    'maxmind_geolite_city_csv',
    'maxmind_geolite_asn_csv',
    'maxmind_geolite_city_web'
  );
