-- Separate signed WebAuthn ceremonies/management from low-entropy TOTP attempts.
-- Existing counters remain intact; old binaries still accept the expanded schema.
ALTER TABLE identity_rate_buckets
    DROP CONSTRAINT identity_rate_buckets_category_check,
    ADD CONSTRAINT identity_rate_buckets_category_check
        CHECK (category IN ('login_account', 'login_source', 'protocol_source', 'mfa_account', 'webauthn_account'));
