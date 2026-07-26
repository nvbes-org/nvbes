CREATE INDEX IF NOT EXISTS idx_workspace_invitations_workspace_created_id
  ON workspace_invitations (workspace_id, created_at DESC, id DESC);
