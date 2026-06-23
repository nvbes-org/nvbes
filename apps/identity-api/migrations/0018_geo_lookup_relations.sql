CREATE TABLE IF NOT EXISTS geo_sources (
  code TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('trusted_header', 'personal_database', 'rdap', 'provider_header', 'stored_profile', 'fallback')),
  trust_level TEXT NOT NULL CHECK (trust_level IN ('none', 'low', 'medium', 'high')),
  priority SMALLINT NOT NULL CHECK (priority > 0),
  base_url TEXT,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO geo_sources (code, kind, trust_level, priority, base_url)
VALUES
  ('trusted_proxy_header', 'trusted_header', 'high', 10, NULL),
  ('personal_database', 'personal_database', 'high', 20, NULL),
  ('arin', 'rdap', 'medium', 30, 'https://rdap.arin.net/registry/ip'),
  ('ripe', 'rdap', 'medium', 40, 'https://rdap.db.ripe.net/ip'),
  ('apnic', 'rdap', 'medium', 50, 'https://rdap.apnic.net/ip'),
  ('lacnic', 'rdap', 'medium', 60, 'https://rdap.lacnic.net/rdap/ip'),
  ('afrinic', 'rdap', 'medium', 70, 'https://rdap.afrinic.net/rdap/ip'),
  ('provider_header', 'provider_header', 'medium', 80, NULL),
  ('remote_lookup', 'rdap', 'medium', 90, NULL),
  ('stored_profile', 'stored_profile', 'low', 100, NULL),
  ('private_network', 'fallback', 'none', 110, NULL),
  ('fallback', 'fallback', 'none', 120, NULL)
ON CONFLICT (code) DO UPDATE SET
  kind = EXCLUDED.kind,
  trust_level = EXCLUDED.trust_level,
  priority = EXCLUDED.priority,
  base_url = EXCLUDED.base_url,
  updated_at = now();

CREATE TABLE IF NOT EXISTS geo_ip_network_relations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  source_code TEXT NOT NULL REFERENCES geo_sources(code),
  relation_key TEXT NOT NULL,
  registry TEXT,
  network CIDR,
  start_ip INET,
  end_ip INET,
  asn BIGINT CHECK (asn IS NULL OR asn > 0),
  organization TEXT,
  country_code CHAR(2) CHECK (country_code IS NULL OR country_code = upper(country_code)),
  source_reference TEXT,
  raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
  fetched_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ,
  CHECK (network IS NOT NULL OR (start_ip IS NOT NULL AND end_ip IS NOT NULL))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_geo_ip_network_relations_unique
  ON geo_ip_network_relations (source_code, relation_key);
CREATE INDEX IF NOT EXISTS idx_geo_ip_network_relations_network
  ON geo_ip_network_relations (network);
CREATE INDEX IF NOT EXISTS idx_geo_ip_network_relations_country
  ON geo_ip_network_relations (country_code);
CREATE INDEX IF NOT EXISTS idx_geo_ip_network_relations_asn
  ON geo_ip_network_relations (asn);

CREATE TABLE IF NOT EXISTS geo_personal_ip_ranges (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  network CIDR NOT NULL UNIQUE,
  country_code CHAR(2) NOT NULL CHECK (country_code = upper(country_code)),
  priority SMALLINT NOT NULL DEFAULT 100 CHECK (priority > 0),
  source_reference TEXT,
  note TEXT,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_geo_personal_ip_ranges_enabled
  ON geo_personal_ip_ranges (enabled, priority);
CREATE INDEX IF NOT EXISTS idx_geo_personal_ip_ranges_network
  ON geo_personal_ip_ranges (network);

CREATE TABLE IF NOT EXISTS geo_lookup_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  purpose TEXT NOT NULL CHECK (purpose IN ('payment', 'security', 'data_region', 'auth', 'audit')),
  subject_type TEXT,
  subject_id UUID,
  request_id TEXT,
  ip_address INET,
  selected_country_code CHAR(2) CHECK (selected_country_code IS NULL OR selected_country_code = upper(selected_country_code)),
  selected_data_region TEXT,
  selected_legal_jurisdiction TEXT,
  selected_source TEXT NOT NULL REFERENCES geo_sources(code),
  confidence TEXT NOT NULL CHECK (confidence IN ('none', 'low', 'medium', 'high')),
  private_network BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_geo_lookup_events_subject
  ON geo_lookup_events (subject_type, subject_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_geo_lookup_events_purpose_created
  ON geo_lookup_events (purpose, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_geo_lookup_events_ip
  ON geo_lookup_events (ip_address);

CREATE TABLE IF NOT EXISTS geo_lookup_evidence (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  lookup_event_id UUID NOT NULL REFERENCES geo_lookup_events(id) ON DELETE CASCADE,
  source_code TEXT NOT NULL REFERENCES geo_sources(code),
  network_relation_id UUID REFERENCES geo_ip_network_relations(id),
  country_code CHAR(2) CHECK (country_code IS NULL OR country_code = upper(country_code)),
  confidence TEXT NOT NULL CHECK (confidence IN ('none', 'low', 'medium', 'high')),
  accepted BOOLEAN NOT NULL,
  reason TEXT NOT NULL,
  relation_snapshot JSONB NOT NULL DEFAULT 'null'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_geo_lookup_evidence_event
  ON geo_lookup_evidence (lookup_event_id);
CREATE INDEX IF NOT EXISTS idx_geo_lookup_evidence_source
  ON geo_lookup_evidence (source_code, accepted);
