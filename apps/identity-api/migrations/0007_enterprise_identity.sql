ALTER TABLE tenant_domains
    ADD COLUMN IF NOT EXISTS sso_required BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS sso_provider_id UUID REFERENCES federated_identity_providers(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_tenant_domains_sso_required
    ON tenant_domains (lower(domain))
    WHERE verified_at IS NOT NULL AND sso_required = TRUE;

ALTER TABLE federated_identity_providers
    ADD COLUMN IF NOT EXISTS provider_family TEXT NOT NULL DEFAULT 'custom';

ALTER TABLE oauth_clients
    ADD COLUMN IF NOT EXISTS requires_admin_consent BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE oauth_scope_metadata
    ADD COLUMN IF NOT EXISTS requires_admin_consent BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE oauth_consents
    ADD COLUMN IF NOT EXISTS granted_by_admin BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE oauth_scope_metadata
SET requires_admin_consent = TRUE
WHERE scope IN ('offline_access')
  AND requires_admin_consent = FALSE;
