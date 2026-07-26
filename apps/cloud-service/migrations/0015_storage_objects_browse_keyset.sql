-- Supports folder-first, case-insensitive name browsing with stable keyset pagination.
CREATE INDEX IF NOT EXISTS idx_storage_objects_browse_keyset
  ON storage_objects (
    workspace_id,
    parent_id,
    (CASE object_type WHEN 'folder' THEN 0 ELSE 1 END),
    lower(name),
    id
  )
  WHERE status IN ('pending', 'active');
