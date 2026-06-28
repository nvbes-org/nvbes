-- Multi-email support for Identity accounts.

ALTER TYPE mfa_factor_type ADD VALUE IF NOT EXISTS 'email';

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS secondary_email_primary_min_age_hours INTEGER NOT NULL DEFAULT 24
        CHECK (secondary_email_primary_min_age_hours BETWEEN 0 AND 8760);

CREATE TABLE IF NOT EXISTS user_email_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    normalized_email VARCHAR(255) NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    CONSTRAINT user_email_addresses_email_not_empty CHECK (length(trim(email)) > 0),
    CONSTRAINT user_email_addresses_normalized_email_not_empty CHECK (length(trim(normalized_email)) > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_email_addresses_active_normalized
    ON user_email_addresses (normalized_email)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_email_addresses_one_primary
    ON user_email_addresses (principal_id)
    WHERE is_primary = TRUE AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_user_email_addresses_principal_active
    ON user_email_addresses (principal_id, created_at DESC)
    WHERE deleted_at IS NULL;

INSERT INTO user_email_addresses (
    principal_id,
    email,
    normalized_email,
    is_primary,
    verified_at,
    created_at,
    updated_at
)
SELECT
    u.principal_id,
    u.email,
    lower(u.email),
    TRUE,
    u.email_verified_at,
    u.created_at,
    u.updated_at
FROM users u
ON CONFLICT DO NOTHING;
