-- Supports role-filtered member browsing and case-insensitive email prefixes.
CREATE INDEX IF NOT EXISTS idx_workspace_memberships_workspace_role_user
  ON workspace_memberships (workspace_id, role, user_id);

CREATE INDEX IF NOT EXISTS idx_users_email_prefix
  ON users (lower(email) text_pattern_ops, id);
