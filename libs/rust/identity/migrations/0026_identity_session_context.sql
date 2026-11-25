ALTER TABLE identity_sessions
    ADD COLUMN context_fingerprint BYTEA
        CHECK (context_fingerprint IS NULL OR octet_length(context_fingerprint) = 32),
    ADD COLUMN reauth_required_at TIMESTAMPTZ;
