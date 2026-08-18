ALTER TABLE account_inbox_events
    ADD COLUMN event_fingerprint BYTEA NOT NULL
        DEFAULT decode(repeat('00', 32), 'hex');

ALTER TABLE account_inbox_events
    ALTER COLUMN event_fingerprint DROP DEFAULT;
