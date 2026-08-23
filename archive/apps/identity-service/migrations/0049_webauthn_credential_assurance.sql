-- Persist WebAuthn credential assurance metadata.
ALTER TABLE mfa_factors
    ADD COLUMN IF NOT EXISTS webauthn_assurance TEXT,
    ADD COLUMN IF NOT EXISTS webauthn_backup_eligible BOOLEAN,
    ADD COLUMN IF NOT EXISTS webauthn_backup_state BOOLEAN,
    ADD COLUMN IF NOT EXISTS webauthn_sign_count BIGINT,
    ADD COLUMN IF NOT EXISTS webauthn_attestation_format TEXT;

ALTER TABLE mfa_factors
    DROP CONSTRAINT IF EXISTS mfa_factors_webauthn_assurance_check;

ALTER TABLE mfa_factors
    ADD CONSTRAINT mfa_factors_webauthn_assurance_check
    CHECK (
        webauthn_assurance IS NULL
        OR webauthn_assurance IN (
            'synced_passkey',
            'device_bound_passkey',
            'hardware_security_key'
        )
    );

UPDATE mfa_factors
SET
    webauthn_backup_eligible = COALESCE(
        webauthn_backup_eligible,
        (factor_data #>> '{passkey,cred,backup_eligible}')::BOOLEAN,
        (factor_data #>> '{passkey,backup_eligible}')::BOOLEAN,
        FALSE
    ),
    webauthn_backup_state = COALESCE(
        webauthn_backup_state,
        (factor_data #>> '{passkey,cred,backup_state}')::BOOLEAN,
        (factor_data #>> '{passkey,backup_state}')::BOOLEAN,
        FALSE
    ),
    webauthn_sign_count = COALESCE(
        webauthn_sign_count,
        (factor_data #>> '{passkey,cred,counter}')::BIGINT,
        (factor_data #>> '{passkey,counter}')::BIGINT,
        0
    ),
    webauthn_attestation_format = COALESCE(
        webauthn_attestation_format,
        factor_data #>> '{passkey,cred,attestation,attestation_format}',
        factor_data #>> '{passkey,attestation,attestation_format}'
    )
WHERE factor_type = 'webauthn';

UPDATE mfa_factors
SET webauthn_assurance = CASE
    WHEN webauthn_backup_eligible THEN 'synced_passkey'
    WHEN factor_data->>'kind' = 'security_key' THEN 'hardware_security_key'
    ELSE 'device_bound_passkey'
END
WHERE factor_type = 'webauthn'
  AND webauthn_assurance IS NULL;
