ALTER TABLE identity_webauthn_credentials
    ADD COLUMN revoked_at TIMESTAMPTZ;
