DELETE FROM privacy_requests
WHERE request_type = 'account_delete';

ALTER TABLE privacy_requests
  DROP CONSTRAINT privacy_requests_check;

ALTER TYPE privacy_request_type RENAME TO privacy_request_type_before_account_closure;

CREATE TYPE privacy_request_type AS ENUM (
  'account_export',
  'workspace_export',
  'workspace_delete'
);

ALTER TABLE privacy_requests
  ALTER COLUMN request_type TYPE privacy_request_type
  USING request_type::text::privacy_request_type;

DROP TYPE privacy_request_type_before_account_closure;

ALTER TABLE privacy_requests
  ADD CONSTRAINT privacy_requests_subject_check CHECK (
    (request_type = 'account_export' AND subject_user_id IS NOT NULL)
    OR
    (request_type IN ('workspace_export', 'workspace_delete') AND workspace_id IS NOT NULL)
  );
