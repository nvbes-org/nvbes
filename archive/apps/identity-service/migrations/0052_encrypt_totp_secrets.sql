-- Add envelope-encrypted storage for TOTP secrets.
ALTER TABLE mfa_factors
    ADD COLUMN IF NOT EXISTS totp_secret_envelope JSONB;

COMMENT ON COLUMN mfa_factors.totp_secret_envelope IS
    'AES-256-GCM envelope encrypted TOTP secret. New writes must not use totp_secret_base32.';
