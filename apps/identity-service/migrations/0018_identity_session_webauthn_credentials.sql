-- Preserve every credential that contributed authentication to a session.
-- Historical sessions have no trustworthy credential attribution; do not infer it.
ALTER TABLE identity_sessions
    ADD CONSTRAINT identity_sessions_id_principal_unique UNIQUE (id, principal_id);
ALTER TABLE identity_webauthn_credentials
    ADD CONSTRAINT identity_webauthn_id_principal_unique UNIQUE (id, principal_id);

CREATE TABLE identity_session_webauthn_credentials (
    session_id UUID NOT NULL,
    credential_id UUID NOT NULL,
    principal_id UUID NOT NULL,
    purpose TEXT NOT NULL CHECK (purpose IN ('primary', 'step_up')),
    authenticated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (session_id, credential_id, purpose),
    FOREIGN KEY (session_id, principal_id)
        REFERENCES identity_sessions(id, principal_id) ON DELETE CASCADE,
    FOREIGN KEY (credential_id, principal_id)
        REFERENCES identity_webauthn_credentials(id, principal_id) ON DELETE CASCADE
);

CREATE INDEX identity_session_webauthn_credential_idx
    ON identity_session_webauthn_credentials (credential_id, session_id);
