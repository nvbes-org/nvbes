-- Retire sessions predating attribution (a later TOTP may hide old WebAuthn use),
-- sessions whose WebAuthn provenance cannot be verified after 0018,
-- and sessions associated with credentials revoked before propagation existed.
WITH revoked AS (
    UPDATE identity_sessions s SET revoked_at=clock_timestamp()
    WHERE s.revoked_at IS NULL AND (
        s.created_at < (SELECT installed_on FROM _sqlx_migrations WHERE version=18) OR
        (s.primary_amr='webauthn' AND NOT EXISTS (
            SELECT 1 FROM identity_session_webauthn_credentials b
            WHERE b.session_id=s.id AND b.purpose='primary'
        )) OR
        (s.step_up_method='webauthn' AND NOT EXISTS (
            SELECT 1 FROM identity_session_webauthn_credentials b
            WHERE b.session_id=s.id AND b.purpose='step_up'
        )) OR EXISTS (
            SELECT 1 FROM identity_session_webauthn_credentials b
            JOIN identity_webauthn_credentials c ON c.id=b.credential_id
            WHERE b.session_id=s.id AND c.revoked_at IS NOT NULL
        )
    )
    RETURNING principal_id
)
INSERT INTO identity_audit_events(id,principal_id,event_type,correlation_id,details)
SELECT gen_random_uuid(),principal_id,'identity.sessions.webauthn_transition',
    gen_random_uuid(),jsonb_build_object('revoked_sessions',count(*))
FROM revoked GROUP BY principal_id;
