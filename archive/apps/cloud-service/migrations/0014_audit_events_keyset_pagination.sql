-- Supports stable workspace-scoped keyset pagination without an extra cursor lookup.
CREATE INDEX IF NOT EXISTS idx_audit_events_workspace_created_at_id
  ON audit_events (workspace_id, created_at DESC, id DESC);
