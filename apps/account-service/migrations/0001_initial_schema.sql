-- Migration: 001_initial_schema.sql
-- =============================================================================
-- Account Service - clean principals-based schema
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TYPE principal_kind AS ENUM ('human', 'service_account', 'bot', 'device');
CREATE TYPE principal_status AS ENUM ('active', 'pending', 'suspended', 'revoked', 'deleted');
CREATE TYPE user_status AS ENUM ('active', 'pending_verification', 'suspended', 'deleted');
CREATE TYPE tenant_kind AS ENUM ('personal', 'team', 'enterprise', 'system');
CREATE TYPE tenant_status AS ENUM ('active', 'suspended', 'deleted');
CREATE TYPE organization_status AS ENUM ('active', 'suspended', 'deleted');
CREATE TYPE workspace_type AS ENUM ('personal', 'team');
CREATE TYPE workspace_member_role AS ENUM ('owner', 'admin', 'member', 'viewer');
CREATE TYPE workspace_member_status AS ENUM ('active', 'invited', 'suspended', 'removed');
CREATE TYPE membership_status AS ENUM ('active', 'invited', 'suspended', 'removed');
CREATE TYPE membership_source AS ENUM ('manual', 'invitation', 'scim', 'jit', 'system');
CREATE TYPE scope_type AS ENUM ('tenant', 'organization', 'workspace');
CREATE TYPE client_type AS ENUM ('confidential', 'public', 'native', 'desktop', 'device', 'service', 'mobile', 'iot');
CREATE TYPE client_policy_status AS ENUM ('active', 'restricted', 'blocked', 'pending_approval');
CREATE TYPE step_up_level AS ENUM ('aal1', 'aal2', 'aal3');
CREATE TYPE mfa_factor_type AS ENUM ('totp', 'webauthn', 'recovery_code');
CREATE TYPE mfa_factor_status AS ENUM ('pending', 'active', 'revoked');
CREATE TYPE device_kind AS ENUM ('desktop', 'mobile', 'tablet', 'security_key', 'iot', 'other');
CREATE TYPE device_trust_level AS ENUM ('unknown', 'remembered', 'managed', 'attested', 'high_assurance');
CREATE TYPE invitation_status AS ENUM ('pending', 'accepted', 'revoked', 'expired');
CREATE TYPE identity_provider_type AS ENUM ('password', 'oidc', 'saml', 'webauthn', 'passkey', 'recovery_code');

CREATE TABLE principals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    principal_kind principal_kind NOT NULL,
    status principal_status NOT NULL DEFAULT 'pending',
    display_name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind tenant_kind NOT NULL DEFAULT 'personal',
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    status tenant_status NOT NULL DEFAULT 'active',
    security_tier TEXT NOT NULL DEFAULT 'standard',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE principals
    ADD CONSTRAINT fk_principals_tenant_id FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    parent_organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    status organization_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, slug)
);

CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    workspace_type workspace_type NOT NULL DEFAULT 'personal',
    plan_code VARCHAR(50) NOT NULL DEFAULT 'solo_pro',
    trial_ends_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE users (
    principal_id UUID PRIMARY KEY REFERENCES principals(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255),
    email_verified_at TIMESTAMPTZ,
    status user_status NOT NULL DEFAULT 'pending_verification',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE service_accounts (
    principal_id UUID PRIMARY KEY REFERENCES principals(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    created_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    description TEXT,
    auth_method TEXT NOT NULL DEFAULT 'private_key_jwt',
    client_id TEXT UNIQUE,
    secret_hash TEXT,
    public_key_jwk JSONB,
    last_rotated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE bots (
    principal_id UUID PRIMARY KEY REFERENCES principals(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    created_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    bot_kind TEXT NOT NULL DEFAULT 'automation',
    app_id UUID,
    status principal_status NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE devices (
    principal_id UUID PRIMARY KEY REFERENCES principals(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    device_kind device_kind NOT NULL,
    device_identifier TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    trust_level device_trust_level NOT NULL DEFAULT 'unknown',
    attestation_provider TEXT,
    attestation_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    public_key_jwk JSONB,
    last_seen_at TIMESTAMPTZ,
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE user_identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    provider_type identity_provider_type NOT NULL,
    provider_id TEXT NOT NULL,
    subject TEXT NOT NULL,
    email TEXT,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider_type, provider_id, subject)
);

CREATE TABLE tenant_memberships (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    principal_kind principal_kind NOT NULL,
    status membership_status NOT NULL DEFAULT 'active',
    source membership_source NOT NULL DEFAULT 'manual',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, principal_id)
);

CREATE TABLE organization_memberships (
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    status membership_status NOT NULL DEFAULT 'active',
    source membership_source NOT NULL DEFAULT 'manual',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (organization_id, principal_id)
);

CREATE TABLE workspace_memberships (
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    role workspace_member_role NOT NULL DEFAULT 'member',
    status workspace_member_status NOT NULL DEFAULT 'active',
    source membership_source NOT NULL DEFAULT 'manual',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_id, principal_id)
);

CREATE TABLE workspace_policies (
    workspace_id UUID PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
    member_can_create_share_links BOOLEAN NOT NULL DEFAULT FALSE,
    require_admin_approval_for_member_share BOOLEAN NOT NULL DEFAULT TRUE,
    default_share_link_ttl_days INTEGER NOT NULL DEFAULT 7,
    max_share_link_ttl_days INTEGER NOT NULL DEFAULT 30,
    required_acr step_up_level NOT NULL DEFAULT 'aal1',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE oauth_clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id VARCHAR(100) NOT NULL UNIQUE,
    client_secret_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    redirect_uris TEXT[] NOT NULL,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    owner_scope_type scope_type NOT NULL,
    owner_scope_id UUID NOT NULL,
    client_type client_type NOT NULL DEFAULT 'confidential',
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE oauth_client_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id UUID NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    scope_type scope_type NOT NULL,
    scope_id UUID NOT NULL,
    allowed_scopes TEXT[] NOT NULL DEFAULT '{}',
    allowed_audiences TEXT[] NOT NULL DEFAULT '{}',
    allowed_resources TEXT[] NOT NULL DEFAULT '{}',
    required_acr step_up_level NOT NULL DEFAULT 'aal1',
    status client_policy_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (client_id, scope_type, scope_id)
);

CREATE TABLE oauth_consents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    client_id UUID NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    scope TEXT[] NOT NULL DEFAULT '{}',
    audience TEXT,
    resource_indicators TEXT[] NOT NULL DEFAULT '{}',
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

CREATE TABLE oauth_scope_metadata (
    scope TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    is_internal BOOLEAN NOT NULL DEFAULT FALSE,
    requires_consent BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE mfa_factors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    factor_type mfa_factor_type NOT NULL,
    status mfa_factor_status NOT NULL DEFAULT 'pending',
    label TEXT,
    totp_secret_base32 TEXT,
    totp_digits SMALLINT NOT NULL DEFAULT 6,
    totp_period_seconds INTEGER NOT NULL DEFAULT 30,
    totp_last_used_counter BIGINT,
    factor_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ
);

CREATE TABLE tenant_domains (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, domain)
);

CREATE TABLE federated_identity_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider_type identity_provider_type NOT NULL,
    name TEXT NOT NULL,
    issuer TEXT,
    metadata_url TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE scim_provisioning_connectors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    base_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE workspace_invitations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    email TEXT NOT NULL,
    role workspace_member_role NOT NULL DEFAULT 'member',
    status invitation_status NOT NULL DEFAULT 'pending',
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    actor_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id UUID,
    ip INET,
    user_agent TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    previous_event_hash TEXT,
    event_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE risk_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    session_id UUID,
    device_id UUID REFERENCES devices(principal_id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    ip_address INET,
    user_agent TEXT,
    risk_score FLOAT NOT NULL DEFAULT 0.0,
    risk_factors JSONB NOT NULL DEFAULT '{}'::jsonb,
    decision TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO oauth_scope_metadata (scope, description, is_internal, requires_consent) VALUES
    ('openid', 'Access your identity.', FALSE, TRUE),
    ('profile', 'Access your profile information.', FALSE, TRUE),
    ('email', 'Access your email address.', FALSE, TRUE),
    ('offline_access', 'Maintain access when you are offline.', FALSE, TRUE)
ON CONFLICT (scope) DO NOTHING;

CREATE INDEX idx_principals_tenant_id ON principals(tenant_id);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_mfa_factors_principal_id ON mfa_factors(principal_id);
CREATE INDEX idx_tenant_memberships_principal_id ON tenant_memberships(principal_id);
CREATE INDEX idx_organization_memberships_principal_id ON organization_memberships(principal_id);
CREATE INDEX idx_workspace_memberships_principal_id ON workspace_memberships(principal_id);
CREATE INDEX idx_user_identities_principal_id ON user_identities(principal_id);
CREATE INDEX idx_oauth_consents_principal_id ON oauth_consents(principal_id);
CREATE INDEX idx_risk_events_principal_id ON risk_events(principal_id);


-- Migration: 002_federation_scaffold.sql
ALTER TABLE tenant_domains
    ADD COLUMN verification_token_hash TEXT,
    ADD COLUMN verification_requested_at TIMESTAMPTZ,
    ADD COLUMN verification_expires_at TIMESTAMPTZ;

CREATE INDEX idx_tenant_domains_tenant_id ON tenant_domains (tenant_id);
CREATE INDEX idx_tenant_domains_verified_at ON tenant_domains (verified_at);



-- Migration: 003_federation_protocol.sql
ALTER TABLE federated_identity_providers
    ADD COLUMN client_id TEXT;



ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS owner_user_id UUID REFERENCES users(principal_id);

-- Migration: 20260511143000_enterprise_password_recovery_v1.sql
CREATE TABLE enterprise_password_recovery_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    available_at TIMESTAMPTZ NOT NULL,
    approved_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    approved_at TIMESTAMPTZ,
    review_available_at TIMESTAMPTZ,
    secondary_approved_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    secondary_approved_at TIMESTAMPTZ,
    reset_token_hash VARCHAR(255),
    reset_token_expires_at TIMESTAMPTZ,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (principal_id)
);

CREATE INDEX idx_enterprise_password_recovery_requests_tenant_id
    ON enterprise_password_recovery_requests (tenant_id, status, available_at DESC);

CREATE INDEX idx_enterprise_password_recovery_requests_email
    ON enterprise_password_recovery_requests (lower(email));


-- Migration: 20260511200000_audit_hash_chain_v1.sql
-- =============================================================================
-- Account Service - Audit Immutability (Hash Chain)
-- =============================================================================

-- Ensure pgcrypto for digest()
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- 1. Function to compute hash chain
CREATE OR REPLACE FUNCTION audit_events_set_hash()
RETURNS TRIGGER AS $$
DECLARE
  previous_hash TEXT;
BEGIN
  -- Get the last hash for this workspace (or tenant if workspace is null)
  -- We partition the chain by tenant_id to ensure a consistent chain per customer
  SELECT event_hash
  INTO previous_hash
  FROM audit_events
  WHERE tenant_id = NEW.tenant_id
  ORDER BY created_at DESC, id DESC
  LIMIT 1;

  NEW.previous_event_hash := previous_hash;
  
  -- Compute hash of the current event including the previous hash
  NEW.event_hash := encode(
    digest(
      concat_ws(
        '|',
        NEW.id::text,
        NEW.tenant_id::text,
        COALESCE(NEW.workspace_id::text, ''),
        COALESCE(NEW.actor_principal_id::text, ''),
        NEW.action,
        NEW.target_type,
        COALESCE(NEW.target_id::text, ''),
        COALESCE(NEW.ip::text, ''),
        COALESCE(NEW.user_agent, ''),
        NEW.metadata::text,
        NEW.created_at::text,
        COALESCE(previous_hash, '')
      ),
      'sha256'
    ),
    'hex'
  );

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. Function to prevent any modification
CREATE OR REPLACE FUNCTION audit_events_prevent_mutation()
RETURNS TRIGGER AS $$
BEGIN
  RAISE EXCEPTION 'audit_events table is append-only and immutable';
END;
$$ LANGUAGE plpgsql;

-- 3. Triggers
DROP TRIGGER IF EXISTS trg_audit_events_set_hash ON audit_events;
CREATE TRIGGER trg_audit_events_set_hash
BEFORE INSERT ON audit_events
FOR EACH ROW
EXECUTE FUNCTION audit_events_set_hash();

DROP TRIGGER IF EXISTS trg_audit_events_prevent_update ON audit_events;
CREATE TRIGGER trg_audit_events_prevent_update
BEFORE UPDATE ON audit_events
FOR EACH ROW
EXECUTE FUNCTION audit_events_prevent_mutation();

DROP TRIGGER IF EXISTS trg_audit_events_prevent_delete ON audit_events;
CREATE TRIGGER trg_audit_events_prevent_delete
BEFORE DELETE ON audit_events
FOR EACH ROW
EXECUTE FUNCTION audit_events_prevent_mutation();

-- 4. Initialization: Backfill existing records if any
-- (Assuming few or no records yet, otherwise this might be slow)
UPDATE audit_events SET event_hash = 'backfill' WHERE event_hash IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_audit_events_event_hash
    ON audit_events (event_hash);


-- Migration: 20260512000000_saml_production_v1.sql
CREATE TABLE saml_pending_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES federated_identity_providers(id) ON DELETE CASCADE,
    relay_state TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_saml_pending_requests_tenant_id ON saml_pending_requests (tenant_id);
CREATE INDEX idx_saml_pending_requests_provider_id ON saml_pending_requests (provider_id);
CREATE INDEX idx_saml_pending_requests_expires_at ON saml_pending_requests (expires_at);

CREATE TABLE saml_assertion_ids (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES federated_identity_providers(id) ON DELETE CASCADE,
    assertion_id TEXT NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, provider_id, assertion_id)
);

CREATE INDEX idx_saml_assertion_ids_tenant_id ON saml_assertion_ids (tenant_id);
CREATE INDEX idx_saml_assertion_ids_provider_id ON saml_assertion_ids (provider_id);
CREATE INDEX idx_saml_assertion_ids_processed_at ON saml_assertion_ids (processed_at);

CREATE TABLE saml_sp_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    entity_id TEXT NOT NULL UNIQUE,
    acs_url TEXT NOT NULL,
    slo_url TEXT,
    signing_cert_pem TEXT,
    signing_key_pem TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, entity_id)
);

CREATE INDEX idx_saml_sp_config_tenant_id ON saml_sp_config (tenant_id);

ALTER TABLE federated_identity_providers
    ADD COLUMN sp_entity_id TEXT,
    ADD COLUMN attribute_mapping JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN encryption_cert_pem TEXT,
    ADD COLUMN require_signed_assertions BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN require_signed_responses BOOLEAN NOT NULL DEFAULT true;

CREATE INDEX idx_federated_identity_providers_sp_entity_id ON federated_identity_providers (sp_entity_id) WHERE sp_entity_id IS NOT NULL;


-- Migration: 20260512100000_rls_multi_tenant_v1.sql
-- =============================================================================
-- Row Level Security - Multi-Tenant Isolation v1
-- =============================================================================

-- Helper function to set GUC variables for RLS context
CREATE OR REPLACE FUNCTION nvbes_set_context(
    p_principal_id UUID DEFAULT NULL,
    p_tenant_id UUID DEFAULT NULL,
    p_workspace_id UUID DEFAULT NULL
) RETURNS void AS $$
BEGIN
    IF p_principal_id IS NOT NULL THEN
        PERFORM set_config('nvbes.principal_id', p_principal_id::text, true);
    END IF;
    IF p_tenant_id IS NOT NULL THEN
        PERFORM set_config('nvbes.tenant_id', p_tenant_id::text, true);
    END IF;
    IF p_workspace_id IS NOT NULL THEN
        PERFORM set_config('nvbes.workspace_id', p_workspace_id::text, true);
    END IF;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Create BYPASSRLS role for system operations
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'nvbes_system') THEN
        CREATE ROLE nvbes_system BYPASSRLS;
    END IF;
END
$$;

-- =============================================================================
-- Tenant-scoped tables: isolation by tenant_id
-- =============================================================================

ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON tenants
    FOR ALL
    USING (id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE principals ENABLE ROW LEVEL SECURITY;
CREATE POLICY principal_isolation ON principals
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
CREATE POLICY organization_isolation ON organizations
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE tenant_memberships ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_membership_isolation ON tenant_memberships
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE organization_memberships ENABLE ROW LEVEL SECURITY;
CREATE POLICY organization_membership_isolation ON organization_memberships
    FOR ALL
    USING (organization_id IN (
        SELECT id FROM organizations
        WHERE tenant_id = current_setting('nvbes.tenant_id')::uuid
    ));

ALTER TABLE tenant_domains ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_domain_isolation ON tenant_domains
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE federated_identity_providers ENABLE ROW LEVEL SECURITY;
CREATE POLICY federated_identity_provider_isolation ON federated_identity_providers
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE scim_provisioning_connectors ENABLE ROW LEVEL SECURITY;
CREATE POLICY scim_provisioning_connector_isolation ON scim_provisioning_connectors
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE oauth_clients ENABLE ROW LEVEL SECURITY;
CREATE POLICY oauth_client_isolation ON oauth_clients
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE oauth_client_policies ENABLE ROW LEVEL SECURITY;
CREATE POLICY oauth_client_policy_isolation ON oauth_client_policies
    FOR ALL
    USING (client_id IN (
        SELECT id FROM oauth_clients
        WHERE tenant_id = current_setting('nvbes.tenant_id')::uuid
    ));

ALTER TABLE enterprise_password_recovery_requests ENABLE ROW LEVEL SECURITY;
CREATE POLICY enterprise_password_recovery_request_isolation ON enterprise_password_recovery_requests
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

-- =============================================================================
-- Workspace-scoped tables: isolation by workspace_id
-- =============================================================================

ALTER TABLE workspaces ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_read_via_membership ON workspaces
    FOR SELECT
    USING (id IN (
        SELECT workspace_id FROM workspace_memberships
        WHERE principal_id = current_setting('nvbes.principal_id')::uuid
        AND status = 'active'
    ));

CREATE POLICY workspace_write ON workspaces
    FOR ALL
    USING (id = current_setting('nvbes.workspace_id')::uuid)
    WITH CHECK (id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE workspace_policies ENABLE ROW LEVEL SECURITY;
CREATE POLICY workspace_policy_isolation ON workspace_policies
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE workspace_memberships ENABLE ROW LEVEL SECURITY;
CREATE POLICY workspace_membership_isolation ON workspace_memberships
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE workspace_invitations ENABLE ROW LEVEL SECURITY;
CREATE POLICY workspace_invitation_isolation ON workspace_invitations
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE oauth_consents ENABLE ROW LEVEL SECURITY;
CREATE POLICY oauth_consent_isolation ON oauth_consents
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE service_accounts ENABLE ROW LEVEL SECURITY;
CREATE POLICY service_account_isolation ON service_accounts
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE bots ENABLE ROW LEVEL SECURITY;
CREATE POLICY bot_isolation ON bots
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

-- Audit events: read isolation (append-only enforced by existing trigger)
ALTER TABLE audit_events ENABLE ROW LEVEL SECURITY;
CREATE POLICY audit_read ON audit_events
    FOR SELECT
    USING (
        workspace_id = current_setting('nvbes.workspace_id')::uuid
        OR tenant_id = current_setting('nvbes.tenant_id')::uuid
    );

-- =============================================================================
-- Tables WITHOUT RLS (global/system tables)
-- =============================================================================
-- oauth_scope_metadata
-- users (principal-scoped, not tenant/workspace)
-- user_identities (principal-scoped)
-- mfa_factors (principal-scoped)
-- devices (principal-scoped)
-- risk_events (principal-scoped)


-- Migration: 20260512200000_jwt_key_rotation_v1.sql
CREATE TABLE signing_keys (
    kid TEXT PRIMARY KEY,
    kms_key_id TEXT,
    algorithm TEXT NOT NULL DEFAULT 'RS256',
    public_key_pem TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    activated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deprecated_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_signing_keys_status ON signing_keys(status) WHERE status IN ('active', 'deprecated');
CREATE INDEX idx_signing_keys_kms ON signing_keys(kms_key_id) WHERE kms_key_id IS NOT NULL;


-- Migration: 20260512210000_email_webhooks_v1.sql
CREATE TYPE email_event_type AS ENUM (
    'email_delivered',
    'email_dropped',
    'email_mailbox_not_found',
    'email_spam',
    'email_deferred',
    'email_bounced',
    'email_blacklisted',
    'email_unsubscribed',
    'email_opened',
    'email_clicked',
    'email_sent'
);

CREATE TABLE email_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_event_id TEXT NOT NULL,
    email TEXT NOT NULL,
    event_type email_event_type NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    details JSONB DEFAULT '{}'::jsonb,
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_email_events_provider_event_id ON email_events(provider_event_id);
CREATE INDEX idx_email_events_email ON email_events(email);
CREATE INDEX idx_email_events_event_type ON email_events(event_type);

CREATE TABLE suppressed_emails (
    email TEXT PRIMARY KEY,
    reason TEXT NOT NULL,
    suppressed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    details JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);


-- Migration: 20260512210001_email_events_provider_email_id.sql
ALTER TABLE email_events
ADD COLUMN provider_email_id TEXT;

CREATE INDEX idx_email_events_provider_email_id ON email_events(provider_email_id);


-- Migration: 20260512220000_email_messages_v1.sql
CREATE TYPE email_message_status AS ENUM (
    'queued',
    'sent',
    'delivered',
    'bounced',
    'complained',
    'dropped'
);

CREATE TABLE email_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL,
    business_type TEXT NOT NULL,
    recipient_email TEXT NOT NULL,
    recipient_hash TEXT NOT NULL,
    provider_email_id TEXT,
    status email_message_status NOT NULL DEFAULT 'queued',
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_email_messages_provider_email_id ON email_messages(provider_email_id);
CREATE INDEX idx_email_messages_recipient_email ON email_messages(recipient_email);
CREATE INDEX idx_email_messages_business_type ON email_messages(business_type);

-- Consolidated V0 migrations that used to live as separate files.

-- Migration: 0004_user_profile_fields.sql
ALTER TABLE users
    ADD COLUMN firstname VARCHAR(255) NOT NULL,
    ADD COLUMN lastname VARCHAR(255) NOT NULL,
    ADD COLUMN username VARCHAR(100),
    ADD COLUMN birthdate DATE,
    ADD COLUMN region CHAR(2);

ALTER TABLE users
    ADD CONSTRAINT users_firstname_not_blank CHECK (btrim(firstname) <> ''),
    ADD CONSTRAINT users_lastname_not_blank CHECK (btrim(lastname) <> '');

CREATE UNIQUE INDEX idx_users_username ON users(username) WHERE username IS NOT NULL;

-- Migration: 0005_multi_region_compliance.sql
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'data_region') THEN
        CREATE TYPE data_region AS ENUM ('eu', 'us', 'ch', 'apac');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'legal_jurisdiction') THEN
        CREATE TYPE legal_jurisdiction AS ENUM ('gdpr', 'ccpa', 'nfdap', 'global');
    END IF;
END $$;

ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS data_region data_region NOT NULL DEFAULT 'eu',
    ADD COLUMN IF NOT EXISTS jurisdiction legal_jurisdiction NOT NULL DEFAULT 'gdpr';

CREATE TABLE IF NOT EXISTS user_consents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    document_version VARCHAR(255) NOT NULL,
    consent_type VARCHAR(255) NOT NULL,
    ip_address INET,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    UNIQUE (principal_id, document_version, consent_type)
);

CREATE INDEX IF NOT EXISTS idx_user_consents_principal_id ON user_consents(principal_id);

-- Migration: 0006_make_user_identity_name_nullable.sql
ALTER TABLE users
    ALTER COLUMN name DROP NOT NULL;

ALTER TABLE principals
    ALTER COLUMN display_name DROP NOT NULL;

-- Migration: 0009_user_settings.sql
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS preferences JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS notifications JSONB NOT NULL DEFAULT '{}';

-- Migration: 0010_idempotency_responses.sql
CREATE TABLE IF NOT EXISTS idempotency_responses (
    idempotency_key TEXT NOT NULL,
    scope TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    response_status INT NOT NULL,
    response_body BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '24 hours',
    PRIMARY KEY (idempotency_key, scope)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_responses_expires_at
    ON idempotency_responses (expires_at);

-- Migration: 0013_oauth_client_assertions.sql
ALTER TABLE oauth_clients
    ADD COLUMN IF NOT EXISTS client_assertion_public_key_jwk JSONB,
    ADD COLUMN IF NOT EXISTS client_assertion_required BOOLEAN NOT NULL DEFAULT FALSE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'oauth_clients_client_assertion_jwk_is_object'
    ) THEN
        ALTER TABLE oauth_clients
            ADD CONSTRAINT oauth_clients_client_assertion_jwk_is_object
            CHECK (
                client_assertion_public_key_jwk IS NULL
                OR jsonb_typeof(client_assertion_public_key_jwk) = 'object'
            );
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS oauth_client_assertion_jtis (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    client_id VARCHAR(100) NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    jti VARCHAR(255) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (client_id, jti)
);

CREATE INDEX IF NOT EXISTS idx_oauth_client_assertion_jtis_expires_at
    ON oauth_client_assertion_jtis(expires_at);

ALTER TABLE oauth_client_assertion_jtis ENABLE ROW LEVEL SECURITY;
CREATE POLICY oauth_client_assertion_jti_isolation ON oauth_client_assertion_jtis
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

-- Migration: 0014_password_history.sql
CREATE TABLE IF NOT EXISTS password_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_password_history_principal_id_created_at ON password_history (principal_id, created_at DESC);

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS password_last_changed_at TIMESTAMPTZ;

UPDATE users SET password_last_changed_at = created_at WHERE password_hash IS NOT NULL;

-- Migration: 0015_pow_challenges.sql
CREATE TABLE IF NOT EXISTS pow_challenges (
    nonce TEXT PRIMARY KEY,
    difficulty INTEGER NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pow_challenges_expires_at ON pow_challenges (expires_at) WHERE consumed_at IS NULL;

-- Migration: 0016_drop_obsolete_rate_limit_and_idempotency_tables.sql
DROP TABLE IF EXISTS idempotency_responses;
DROP TABLE IF EXISTS rate_limit_events;
