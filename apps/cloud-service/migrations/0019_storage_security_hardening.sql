-- Runtime connections must SET ROLE nvbes_app after migrations.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'nvbes_app') THEN
    CREATE ROLE nvbes_app
      NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
  END IF;

  ALTER ROLE nvbes_app
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;

  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'nvbes_system') THEN
    CREATE ROLE nvbes_system
      NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION BYPASSRLS;
  END IF;

  ALTER ROLE nvbes_system
    NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION BYPASSRLS;

  EXECUTE format('GRANT nvbes_app, nvbes_system TO %I', current_user);
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'nvbes_runtime') THEN
    GRANT nvbes_app TO nvbes_runtime;
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'nvbes_worker') THEN
    GRANT nvbes_system TO nvbes_worker;
  END IF;
  EXECUTE format('GRANT CONNECT ON DATABASE %I TO nvbes_app, nvbes_system', current_database());
END
$$;

CREATE TABLE data_retention_policies (
  category TEXT PRIMARY KEY,
  classification data_classification NOT NULL,
  active_retention_days INTEGER NOT NULL CHECK (active_retention_days >= 0),
  deleted_retention_days INTEGER NOT NULL CHECK (deleted_retention_days >= 0),
  legal_hold_allowed BOOLEAN NOT NULL DEFAULT true,
  cryptographic_erasure_required BOOLEAN NOT NULL DEFAULT false,
  description TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO data_retention_policies (
  category,
  classification,
  active_retention_days,
  deleted_retention_days,
  legal_hold_allowed,
  cryptographic_erasure_required,
  description
)
VALUES
  ('public_content', 'public', 365, 30, true, false, 'Published customer content.'),
  ('service_metadata', 'internal', 365, 90, true, false, 'Operational metadata required to run the service.'),
  ('customer_content', 'confidential', 365, 30, true, true, 'Private customer files and document content.'),
  ('identity_secret', 'restricted', 90, 0, false, true, 'Authentication secrets and recoverable credentials.'),
  ('security_audit', 'restricted', 2190, 90, true, false, 'Security and access audit evidence.'),
  ('quarantine', 'restricted', 30, 0, true, true, 'Rejected or malware-infected uploads.')
ON CONFLICT (category) DO UPDATE
SET classification = EXCLUDED.classification,
    active_retention_days = EXCLUDED.active_retention_days,
    deleted_retention_days = EXCLUDED.deleted_retention_days,
    legal_hold_allowed = EXCLUDED.legal_hold_allowed,
    cryptographic_erasure_required = EXCLUDED.cryptographic_erasure_required,
    description = EXCLUDED.description,
    updated_at = NOW();

ALTER TABLE storage_objects
  ADD COLUMN IF NOT EXISTS retention_category TEXT NOT NULL DEFAULT 'customer_content'
    REFERENCES data_retention_policies(category),
  ADD COLUMN IF NOT EXISTS retention_until TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS cryptographic_erased_at TIMESTAMPTZ;

CREATE TABLE storage_object_key_envelopes (
  storage_object_id UUID PRIMARY KEY REFERENCES storage_objects(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  kek_id TEXT NOT NULL,
  kek_version INTEGER NOT NULL CHECK (kek_version > 0),
  wrapped_dek JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  rotated_at TIMESTAMPTZ,
  destroyed_at TIMESTAMPTZ
);

CREATE INDEX idx_storage_object_key_envelopes_workspace
  ON storage_object_key_envelopes(workspace_id, storage_object_id)
  WHERE destroyed_at IS NULL;

ALTER TABLE storage_object_key_envelopes ENABLE ROW LEVEL SECURITY;
ALTER TABLE storage_object_key_envelopes FORCE ROW LEVEL SECURITY;
CREATE POLICY storage_object_key_envelope_isolation ON storage_object_key_envelopes
  FOR ALL
  USING (workspace_id = current_setting('nvbes.workspace_id')::uuid)
  WITH CHECK (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE workspace_memberships ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_memberships FORCE ROW LEVEL SECURITY;
CREATE POLICY workspace_membership_v2_isolation ON workspace_memberships
  FOR ALL
  USING (
    workspace_id = current_setting('nvbes.workspace_id')::uuid
    OR user_id = current_setting('nvbes.user_id')::uuid
  )
  WITH CHECK (workspace_id = current_setting('nvbes.workspace_id')::uuid);

DROP POLICY IF EXISTS workspace_read_via_membership ON workspaces;
CREATE POLICY workspace_read_via_membership ON workspaces
  FOR SELECT
  USING (
    EXISTS (
      SELECT 1
      FROM workspace_memberships membership
      WHERE membership.workspace_id = workspaces.id
        AND membership.user_id = current_setting('nvbes.user_id')::uuid
        AND membership.status = 'active'
    )
  );

CREATE OR REPLACE FUNCTION cryptographically_erase_storage_object(
  p_workspace_id UUID,
  p_storage_object_id UUID
) RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path = public, pg_temp
AS $$
DECLARE
  deleted_keys BIGINT;
BEGIN
  IF p_workspace_id <> current_setting('nvbes.workspace_id')::uuid THEN
    RAISE EXCEPTION 'workspace context mismatch' USING ERRCODE = '42501';
  END IF;

  DELETE FROM storage_object_key_envelopes
  WHERE workspace_id = p_workspace_id
    AND storage_object_id = p_storage_object_id
    AND destroyed_at IS NULL;
  GET DIAGNOSTICS deleted_keys = ROW_COUNT;

  UPDATE storage_objects
  SET cryptographic_erased_at = COALESCE(cryptographic_erased_at, NOW()),
      updated_at = NOW()
  WHERE workspace_id = p_workspace_id
    AND id = p_storage_object_id;

  RETURN deleted_keys > 0;
END;
$$;

CREATE OR REPLACE FUNCTION resolve_public_share_workspace(p_token_hash TEXT)
RETURNS UUID
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
  SELECT workspace_id
  FROM share_links
  WHERE token_hash = p_token_hash
  LIMIT 1
$$;

ALTER FUNCTION resolve_public_share_workspace(TEXT) OWNER TO nvbes_system;

ALTER TABLE tenants FORCE ROW LEVEL SECURITY;
ALTER TABLE organizations FORCE ROW LEVEL SECURITY;
ALTER TABLE tenant_memberships FORCE ROW LEVEL SECURITY;
ALTER TABLE organization_memberships FORCE ROW LEVEL SECURITY;
ALTER TABLE workspaces FORCE ROW LEVEL SECURITY;
ALTER TABLE workspace_policies FORCE ROW LEVEL SECURITY;
ALTER TABLE workspace_members FORCE ROW LEVEL SECURITY;
ALTER TABLE workspace_invitations FORCE ROW LEVEL SECURITY;
ALTER TABLE quota_usage FORCE ROW LEVEL SECURITY;
ALTER TABLE usage_events FORCE ROW LEVEL SECURITY;
ALTER TABLE sessions FORCE ROW LEVEL SECURITY;
ALTER TABLE api_keys FORCE ROW LEVEL SECURITY;
ALTER TABLE api_request_logs FORCE ROW LEVEL SECURITY;
ALTER TABLE storage_objects FORCE ROW LEVEL SECURITY;
ALTER TABLE upload_sessions FORCE ROW LEVEL SECURITY;
ALTER TABLE upload_parts FORCE ROW LEVEL SECURITY;
ALTER TABLE share_links FORCE ROW LEVEL SECURITY;
ALTER TABLE privacy_requests FORCE ROW LEVEL SECURITY;
ALTER TABLE audit_events FORCE ROW LEVEL SECURITY;
ALTER TABLE api_key_nonces FORCE ROW LEVEL SECURITY;
ALTER TABLE billing_entitlement_snapshots FORCE ROW LEVEL SECURITY;

GRANT USAGE ON SCHEMA public TO nvbes_app, nvbes_system;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO nvbes_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO nvbes_app;
REVOKE INSERT, UPDATE, DELETE ON data_retention_policies FROM nvbes_app;
REVOKE UPDATE, DELETE ON audit_events FROM nvbes_app;
GRANT SELECT ON data_retention_policies TO nvbes_app;
GRANT SELECT, INSERT ON audit_events TO nvbes_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO nvbes_system;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO nvbes_system;
GRANT EXECUTE ON FUNCTION cryptographically_erase_storage_object(UUID, UUID) TO nvbes_app;
GRANT EXECUTE ON FUNCTION resolve_public_share_workspace(TEXT) TO nvbes_app;

ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO nvbes_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO nvbes_app;
