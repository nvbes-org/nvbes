-- Existing sessions were created exclusively by the password authenticator.
ALTER TABLE identity_sessions
    ADD COLUMN authenticated_at TIMESTAMPTZ,
    ADD COLUMN primary_amr TEXT,
    ADD COLUMN step_up_at TIMESTAMPTZ;
UPDATE identity_sessions SET authenticated_at=created_at, primary_amr='pwd';
-- Do not invent a proof timestamp for older step-ups: require a fresh proof.
UPDATE identity_sessions SET step_up_method=NULL, step_up_expires_at=NULL;
ALTER TABLE identity_sessions
    ALTER COLUMN authenticated_at SET NOT NULL,
    ALTER COLUMN primary_amr SET NOT NULL,
    ADD CONSTRAINT identity_sessions_primary_amr CHECK (primary_amr IN ('pwd','webauthn')),
    ADD CONSTRAINT identity_sessions_step_up_timestamp CHECK (
        (step_up_at IS NULL) = (step_up_expires_at IS NULL)
        AND (step_up_at IS NULL OR step_up_expires_at > step_up_at)
    );

-- Authentication evidence is captured at authorization, never from client input
-- or from a later change to the session. In-flight old codes have no evidence
-- and must be refused. No default invents evidence for future insertions.
ALTER TABLE identity_oauth_codes ADD COLUMN authentication JSONB
    CHECK (jsonb_typeof(authentication) = 'object');
ALTER TABLE identity_oauth_grants ADD COLUMN token_issued_at TIMESTAMPTZ;
