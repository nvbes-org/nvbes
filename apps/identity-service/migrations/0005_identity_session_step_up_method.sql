ALTER TABLE identity_sessions
    ADD COLUMN step_up_method TEXT
    CHECK (step_up_method IN ('totp', 'webauthn'));

ALTER TABLE identity_sessions
    ADD CONSTRAINT identity_sessions_step_up_method_matches_expiry
    CHECK ((step_up_expires_at IS NULL) = (step_up_method IS NULL));
