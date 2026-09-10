ALTER TABLE identity_webauthn_challenges
    ADD COLUMN session_id UUID REFERENCES identity_sessions(id) ON DELETE CASCADE;

-- Legacy public-key-only rows cannot authenticate until re-enrollment. The
-- complete library credential carries verification policy, counters and flags.
ALTER TABLE identity_webauthn_credentials
    ADD COLUMN passkey JSONB CHECK (passkey IS NULL OR jsonb_typeof(passkey) = 'object'),
    ALTER COLUMN discoverable DROP DEFAULT,
    ALTER COLUMN discoverable DROP NOT NULL;
