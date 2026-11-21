ALTER TABLE identity_webauthn_challenges
    ADD COLUMN ceremony_state JSONB
    CHECK (ceremony_state IS NULL OR jsonb_typeof(ceremony_state) = 'object');
