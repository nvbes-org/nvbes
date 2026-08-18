CREATE TABLE identity_outbox_events (
    id UUID PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    publish_attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at TIMESTAMPTZ,
    dead_lettered_at TIMESTAMPTZ,
    last_error_code VARCHAR(64),
    last_error_summary VARCHAR(256),
    CONSTRAINT identity_outbox_publish_attempts_non_negative
        CHECK (publish_attempts >= 0)
);

CREATE UNIQUE INDEX identity_outbox_account_registration_once
    ON identity_outbox_events (aggregate_id, event_type)
    WHERE event_type = 'identity.principal.registered.v1';

CREATE INDEX identity_outbox_dispatchable
    ON identity_outbox_events (next_attempt_at, occurred_at, id)
    WHERE dead_lettered_at IS NULL;
