DELETE FROM privacy_requests
WHERE request_type::text = 'account_export';

ALTER TABLE privacy_requests
  DROP CONSTRAINT IF EXISTS privacy_requests_subject_check;

ALTER TYPE privacy_request_type RENAME TO privacy_request_type_before_account_export_removal;

CREATE TYPE privacy_request_type AS ENUM (
  'workspace_export',
  'workspace_delete'
);

ALTER TABLE privacy_requests
  ALTER COLUMN request_type TYPE privacy_request_type
  USING request_type::text::privacy_request_type;

DROP TYPE privacy_request_type_before_account_export_removal;

ALTER TABLE privacy_requests
  ADD CONSTRAINT privacy_requests_subject_scope_check CHECK (
    request_type IN ('workspace_export', 'workspace_delete')
    AND workspace_id IS NOT NULL
  );
