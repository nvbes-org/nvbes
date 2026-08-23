INSERT INTO geo_sources (code, kind, trust_level, priority, base_url)
VALUES ('v2fly_geoip', 'rdap', 'medium', 85, 'https://github.com/v2fly/geoip')
ON CONFLICT (code) DO UPDATE SET
  kind = EXCLUDED.kind,
  trust_level = EXCLUDED.trust_level,
  priority = EXCLUDED.priority,
  base_url = EXCLUDED.base_url,
  enabled = TRUE,
  updated_at = now();

CREATE INDEX IF NOT EXISTS idx_geo_ip_network_relations_v2fly_freshness
  ON geo_ip_network_relations (source_code, fetched_at DESC, expires_at)
  WHERE source_code = 'v2fly_geoip';
