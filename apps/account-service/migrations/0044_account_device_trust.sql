CREATE TABLE account_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    installation_token_hash TEXT NOT NULL,
    profile_hash TEXT,
    profile_version SMALLINT NOT NULL DEFAULT 1,
    display_name TEXT NOT NULL,
    trust_level TEXT NOT NULL DEFAULT 'unknown'
        CHECK (trust_level IN ('unknown', 'recognized', 'trusted', 'restricted', 'revoked')),
    trust_score SMALLINT NOT NULL DEFAULT 0 CHECK (trust_score BETWEEN 0 AND 100),
    successful_auth_count INTEGER NOT NULL DEFAULT 0,
    failed_auth_count INTEGER NOT NULL DEFAULT 0,
    last_network_hash TEXT,
    last_country_code TEXT,
    last_user_agent_family TEXT,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_verified_at TIMESTAMPTZ,
    trusted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (principal_id, installation_token_hash)
);

CREATE INDEX idx_account_devices_principal_last_seen
    ON account_devices (principal_id, last_seen_at DESC)
    WHERE revoked_at IS NULL;

CREATE INDEX idx_account_devices_principal_trust
    ON account_devices (principal_id, trust_level, trust_score DESC);
