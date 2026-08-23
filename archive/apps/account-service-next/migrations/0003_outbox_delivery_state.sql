ALTER TABLE account_outbox_events
    ADD COLUMN next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN claimed_at TIMESTAMPTZ,
    ADD COLUMN dead_lettered_at TIMESTAMPTZ,
    ADD COLUMN last_error_code VARCHAR(64),
    ADD COLUMN last_error_summary VARCHAR(256);

DROP INDEX account_outbox_events_unpublished;

CREATE INDEX account_outbox_events_dispatchable
    ON account_outbox_events (next_attempt_at, occurred_at, id)
    WHERE published_at IS NULL AND dead_lettered_at IS NULL;
