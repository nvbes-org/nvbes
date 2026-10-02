ALTER TABLE identity_webauthn_challenges
    ADD COLUMN oauth_request_hash BYTEA
        REFERENCES identity_oauth_requests(handle_hash) ON DELETE CASCADE;

CREATE UNIQUE INDEX identity_webauthn_oauth_ceremony_idx
    ON identity_webauthn_challenges(oauth_request_hash)
    WHERE oauth_request_hash IS NOT NULL AND purpose='authentication';
