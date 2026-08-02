CREATE TABLE identity_oidc_profile_claims (
    principal_id UUID PRIMARY KEY REFERENCES principals(id) ON DELETE CASCADE,
    display_name VARCHAR(201) NOT NULL DEFAULT 'User',
    given_name VARCHAR(100),
    family_name VARCHAR(100),
    preferred_username VARCHAR(100),
    birthdate DATE,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    profile_version BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT identity_oidc_profile_display_name_not_blank
        CHECK (btrim(display_name) <> ''),
    CONSTRAINT identity_oidc_profile_given_name_bounded
        CHECK (given_name IS NULL OR char_length(given_name) BETWEEN 1 AND 100),
    CONSTRAINT identity_oidc_profile_family_name_bounded
        CHECK (family_name IS NULL OR char_length(family_name) BETWEEN 1 AND 100),
    CONSTRAINT identity_oidc_profile_username_bounded
        CHECK (preferred_username IS NULL OR char_length(preferred_username) BETWEEN 1 AND 100),
    CONSTRAINT identity_oidc_profile_version_non_negative
        CHECK (profile_version >= 0)
);

INSERT INTO identity_oidc_profile_claims (
    principal_id,
    display_name,
    given_name,
    family_name,
    preferred_username,
    birthdate,
    projected_at,
    profile_version
)
SELECT
    principal_id,
    COALESCE(
        NULLIF(btrim(concat_ws(' ', firstname, lastname)), ''),
        NULLIF(btrim(username), ''),
        'User'
    ),
    NULLIF(btrim(firstname), ''),
    NULLIF(btrim(lastname), ''),
    NULLIF(btrim(username), ''),
    birthdate,
    updated_at,
    0
FROM users;

CREATE TABLE identity_inbox_events (
    event_id UUID PRIMARY KEY,
    event_type TEXT NOT NULL,
    principal_id UUID NOT NULL,
    event_fingerprint BYTEA NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX identity_inbox_events_principal
    ON identity_inbox_events (principal_id, processed_at DESC);

DROP INDEX IF EXISTS idx_users_username;

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_firstname_not_blank,
    DROP CONSTRAINT IF EXISTS users_lastname_not_blank,
    DROP COLUMN firstname,
    DROP COLUMN lastname,
    DROP COLUMN username,
    DROP COLUMN birthdate,
    DROP COLUMN region;
