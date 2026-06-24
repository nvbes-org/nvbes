ALTER TABLE api_request_logs
  ADD COLUMN IF NOT EXISTS geo_country_code TEXT,
  ADD COLUMN IF NOT EXISTS geo_source TEXT,
  ADD COLUMN IF NOT EXISTS geo_confidence TEXT,
  ADD COLUMN IF NOT EXISTS geo_network_kind TEXT
    CHECK (geo_network_kind IS NULL OR geo_network_kind IN ('unknown', 'residential', 'mobile', 'datacenter', 'vpn', 'proxy', 'tor')),
  ADD COLUMN IF NOT EXISTS geo_risk_score SMALLINT
    CHECK (geo_risk_score IS NULL OR (geo_risk_score >= 0 AND geo_risk_score <= 100)),
  ADD COLUMN IF NOT EXISTS geo_risk_labels TEXT[] NOT NULL DEFAULT '{}'::TEXT[];

CREATE INDEX IF NOT EXISTS idx_api_request_logs_geo_kind_score_created_at
  ON api_request_logs (geo_network_kind, geo_risk_score DESC, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_api_request_logs_geo_risk_labels
  ON api_request_logs USING GIN (geo_risk_labels);
