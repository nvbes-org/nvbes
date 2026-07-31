INSERT INTO geo_sources (code, kind, trust_level, priority, base_url)
VALUES ('ip_intelligence', 'rdap', 'medium', 80, NULL)
ON CONFLICT (code) DO UPDATE SET
  kind = EXCLUDED.kind,
  trust_level = EXCLUDED.trust_level,
  priority = EXCLUDED.priority,
  base_url = EXCLUDED.base_url,
  updated_at = now();

ALTER TABLE geo_ip_network_relations
  ADD COLUMN IF NOT EXISTS network_kind TEXT NOT NULL DEFAULT 'unknown'
    CHECK (network_kind IN ('unknown', 'residential', 'mobile', 'datacenter', 'vpn', 'proxy', 'tor')),
  ADD COLUMN IF NOT EXISTS risk_score SMALLINT NOT NULL DEFAULT 50
    CHECK (risk_score >= 0 AND risk_score <= 100),
  ADD COLUMN IF NOT EXISTS risk_labels TEXT[] NOT NULL DEFAULT ARRAY['unknown']::TEXT[];

ALTER TABLE geo_lookup_events
  ADD COLUMN IF NOT EXISTS network_kind TEXT NOT NULL DEFAULT 'unknown'
    CHECK (network_kind IN ('unknown', 'residential', 'mobile', 'datacenter', 'vpn', 'proxy', 'tor')),
  ADD COLUMN IF NOT EXISTS risk_score SMALLINT NOT NULL DEFAULT 50
    CHECK (risk_score >= 0 AND risk_score <= 100),
  ADD COLUMN IF NOT EXISTS risk_labels TEXT[] NOT NULL DEFAULT ARRAY['unknown']::TEXT[];

ALTER TABLE geo_personal_ip_ranges
  ADD COLUMN IF NOT EXISTS network_kind TEXT NOT NULL DEFAULT 'residential'
    CHECK (network_kind IN ('unknown', 'residential', 'mobile', 'datacenter', 'vpn', 'proxy', 'tor')),
  ADD COLUMN IF NOT EXISTS risk_score SMALLINT NOT NULL DEFAULT 15
    CHECK (risk_score >= 0 AND risk_score <= 100),
  ADD COLUMN IF NOT EXISTS risk_labels TEXT[] NOT NULL DEFAULT ARRAY['personal_database', 'residential']::TEXT[];

CREATE INDEX IF NOT EXISTS idx_geo_ip_network_relations_kind_score
  ON geo_ip_network_relations (network_kind, risk_score DESC);
CREATE INDEX IF NOT EXISTS idx_geo_lookup_events_kind_score
  ON geo_lookup_events (network_kind, risk_score DESC, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_geo_personal_ip_ranges_kind_score
  ON geo_personal_ip_ranges (network_kind, risk_score DESC);
