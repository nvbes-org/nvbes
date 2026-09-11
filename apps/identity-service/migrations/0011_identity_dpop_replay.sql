CREATE TABLE identity_dpop_replay_keys (
    jti_hash BYTEA NOT NULL CHECK (octet_length(jti_hash) = 32),
    jkt_hash BYTEA NOT NULL CHECK (octet_length(jkt_hash) = 32),
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (jti_hash, jkt_hash)
);
CREATE INDEX identity_dpop_replay_expiry_idx ON identity_dpop_replay_keys(expires_at);
