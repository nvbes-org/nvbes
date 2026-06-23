ALTER TABLE workspace_policies
  ADD COLUMN public_api_network_policy_mode TEXT NOT NULL DEFAULT 'enforce'
    CHECK (public_api_network_policy_mode IN ('enforce', 'monitor_only', 'disabled')),
  ADD COLUMN public_api_block_vpn BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN public_api_block_proxy BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN public_api_block_tor BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN public_api_block_datacenter BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN public_api_high_risk_score_threshold SMALLINT NOT NULL DEFAULT 90
    CHECK (public_api_high_risk_score_threshold BETWEEN 0 AND 100);

CREATE TABLE public_api_network_allowlist (
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  cidr CIDR NOT NULL,
  reason TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ,
  PRIMARY KEY (workspace_id, cidr)
);

CREATE INDEX idx_public_api_network_allowlist_workspace_expires
  ON public_api_network_allowlist (workspace_id, expires_at);

ALTER TABLE geo_lookup_events
  DROP CONSTRAINT IF EXISTS geo_lookup_events_purpose_check;

ALTER TABLE geo_lookup_events
  ADD CONSTRAINT geo_lookup_events_purpose_check
    CHECK (purpose IN (
      'payment',
      'security',
      'data_region',
      'auth',
      'audit',
      'drive_api',
      'drive_audit',
      'identity_audit'
    ));
