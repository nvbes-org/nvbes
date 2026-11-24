CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS geo_sources (
  code TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN (
    'trusted_header', 'personal_database', 'rdap', 'provider_header', 'stored_profile', 'fallback'
  )),
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
  ('ip_intelligence', 'rdap', 'medium', 80, NULL),
  ('maxmind_geolite_country_csv', 'rdap', 'medium', 82, 'https://www.maxmind.com/en/geolite-free-ip-geolocation-data'),
  ('maxmind_geolite_city_csv', 'rdap', 'medium', 81, 'https://www.maxmind.com/en/geolite-free-ip-geolocation-data'),
  ('maxmind_geolite_asn_csv', 'rdap', 'medium', 84, 'https://www.maxmind.com/en/geolite-free-ip-geolocation-data'),
  ('maxmind_geolite_city_web', 'rdap', 'medium', 83, 'https://geolite.info/geoip/v2.1/city'),
  ('v2fly_geoip', 'rdap', 'medium', 85, 'https://github.com/v2fly/geoip'),
  ('loyalsoldier_geoip', 'rdap', 'medium', 86, 'https://github.com/Loyalsoldier/geoip'),
  ('provider_header', 'provider_header', 'medium', 90, NULL),
  ('remote_lookup', 'rdap', 'medium', 100, NULL),
  ('stored_profile', 'stored_profile', 'low', 110, NULL),
  ('private_network', 'fallback', 'none', 120, NULL),
  ('fallback', 'fallback', 'none', 130, NULL)
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
  network_kind TEXT NOT NULL DEFAULT 'unknown'
    CHECK (network_kind IN ('unknown', 'residential', 'mobile', 'datacenter', 'vpn', 'proxy', 'tor')),
  risk_score SMALLINT NOT NULL DEFAULT 50 CHECK (risk_score >= 0 AND risk_score <= 100),
  risk_labels TEXT[] NOT NULL DEFAULT ARRAY['unknown']::TEXT[],
  raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
  fetched_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ,
  CHECK (network IS NOT NULL OR (start_ip IS NOT NULL AND end_ip IS NOT NULL))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_geo_ip_network_relations_unique
  ON geo_ip_network_relations (source_code, relation_key);

CREATE TABLE IF NOT EXISTS geo_personal_ip_ranges (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  network CIDR NOT NULL UNIQUE,
  country_code CHAR(2) NOT NULL CHECK (country_code = upper(country_code)),
  priority SMALLINT NOT NULL DEFAULT 100 CHECK (priority > 0),
  source_reference TEXT,
  note TEXT,
  network_kind TEXT NOT NULL DEFAULT 'residential'
    CHECK (network_kind IN ('unknown', 'residential', 'mobile', 'datacenter', 'vpn', 'proxy', 'tor')),
  risk_score SMALLINT NOT NULL DEFAULT 15 CHECK (risk_score >= 0 AND risk_score <= 100),
  risk_labels TEXT[] NOT NULL DEFAULT ARRAY['personal_database', 'residential']::TEXT[],
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS geo_lookup_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  purpose TEXT NOT NULL CHECK (purpose IN (
    'payment', 'security', 'data_region', 'auth', 'audit',
    'drive_api', 'drive_audit', 'identity_audit'
  )),
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
  network_kind TEXT NOT NULL DEFAULT 'unknown'
    CHECK (network_kind IN ('unknown', 'residential', 'mobile', 'datacenter', 'vpn', 'proxy', 'tor')),
  risk_score SMALLINT NOT NULL DEFAULT 50 CHECK (risk_score >= 0 AND risk_score <= 100),
  risk_labels TEXT[] NOT NULL DEFAULT ARRAY['unknown']::TEXT[],
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

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
