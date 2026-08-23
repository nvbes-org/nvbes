-- Supports recent-first, workspace-scoped trash pagination.
CREATE INDEX IF NOT EXISTS idx_storage_objects_trash_keyset
  ON storage_objects (workspace_id, trashed_at DESC, id DESC)
  WHERE status = 'trashed';
