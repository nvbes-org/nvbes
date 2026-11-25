-- Only encrypted credential email commands may be retained here.
CREATE TABLE identity_recovery_deliveries (
    challenge_id UUID PRIMARY KEY REFERENCES identity_recovery_challenges(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES identity_principals(id) ON DELETE CASCADE,
    ciphertext BYTEA,
    nonce BYTEA,
    key_version SMALLINT,
    state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','sending','accepted','failed','expired','cancelled')),
    attempts SMALLINT NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 8),
    available_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    deliver_before TIMESTAMPTZ NOT NULL,
    lease_token UUID,
    lease_expires_at TIMESTAMPTZ,
    outcome TEXT,
    settled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CHECK ((state IN ('pending','sending') AND ciphertext IS NOT NULL AND octet_length(ciphertext) BETWEEN 17 AND 16400 AND nonce IS NOT NULL AND octet_length(nonce)=12 AND key_version IS NOT NULL AND key_version>0)
        OR (state NOT IN ('pending','sending') AND ciphertext IS NULL AND nonce IS NULL AND key_version IS NULL)),
    CHECK ((state='sending' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)
        OR (state<>'sending' AND lease_token IS NULL AND lease_expires_at IS NULL)),
    CHECK ((state IN ('pending','sending') AND settled_at IS NULL)
        OR (state NOT IN ('pending','sending') AND settled_at IS NOT NULL))
);

CREATE INDEX identity_recovery_deliveries_pending_idx
    ON identity_recovery_deliveries(available_at) WHERE state IN ('pending','sending');
