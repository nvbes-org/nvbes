-- Local to Billing; no shared database with Identity or Account.
-- The verifier serializes each bucket and retains at most 256 rows per bucket.
CREATE TABLE resource_dpop_replays (
    bucket SMALLINT NOT NULL CHECK (bucket BETWEEN 0 AND 63),
    jkt_hash BYTEA NOT NULL CHECK (octet_length(jkt_hash)=32),
    jti_hash BYTEA NOT NULL CHECK (octet_length(jti_hash)=32),
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (bucket, jkt_hash, jti_hash)
);
CREATE INDEX resource_dpop_replays_expiry ON resource_dpop_replays(bucket, expires_at);
