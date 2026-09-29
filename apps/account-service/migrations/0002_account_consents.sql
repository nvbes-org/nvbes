CREATE TABLE account_consents (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES account_profiles(principal_id) ON DELETE CASCADE,
    consent_type TEXT NOT NULL CHECK (char_length(consent_type) BETWEEN 1 AND 100),
    document_version TEXT NOT NULL CHECK (char_length(document_version) BETWEEN 1 AND 100),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    revoked_at TIMESTAMPTZ,
    CHECK (revoked_at IS NULL OR revoked_at >= granted_at)
);

CREATE UNIQUE INDEX account_consents_active_unique
    ON account_consents (principal_id, consent_type, document_version)
    WHERE revoked_at IS NULL;

CREATE INDEX account_consents_principal_history
    ON account_consents (principal_id, granted_at DESC, id DESC);
