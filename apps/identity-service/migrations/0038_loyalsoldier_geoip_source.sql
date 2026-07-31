INSERT INTO geo_sources (code, kind, trust_level, priority, base_url)
VALUES (
  'loyalsoldier_geoip',
  'rdap',
  'medium',
  86,
  'https://github.com/Loyalsoldier/geoip'
)
ON CONFLICT (code) DO UPDATE SET
  kind = EXCLUDED.kind,
  trust_level = EXCLUDED.trust_level,
  priority = EXCLUDED.priority,
  base_url = EXCLUDED.base_url,
  enabled = TRUE,
  updated_at = now();

CREATE INDEX IF NOT EXISTS idx_geo_ip_network_relations_loyalsoldier_freshness
  ON geo_ip_network_relations (source_code, fetched_at DESC, expires_at)
  WHERE source_code = 'loyalsoldier_geoip';
