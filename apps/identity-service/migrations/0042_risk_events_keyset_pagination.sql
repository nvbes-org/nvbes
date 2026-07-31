-- Supports stable keyset pagination within each principal's security-event stream.
CREATE INDEX IF NOT EXISTS idx_risk_events_principal_created_at_id
  ON risk_events (principal_id, created_at DESC, id DESC);
