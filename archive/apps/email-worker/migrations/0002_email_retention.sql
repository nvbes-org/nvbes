ALTER TABLE email_messages
    ALTER COLUMN recipient_ciphertext DROP NOT NULL,
    ALTER COLUMN recipient_nonce DROP NOT NULL,
    ALTER COLUMN template_ciphertext DROP NOT NULL,
    ALTER COLUMN template_nonce DROP NOT NULL,
    ADD COLUMN payload_purged_at TIMESTAMPTZ;

ALTER TABLE email_provider_events
    ADD COLUMN diagnostic_purged_at TIMESTAMPTZ;

ALTER TABLE email_suppressions
    DROP CONSTRAINT email_suppressions_source_event_id_fkey,
    ADD CONSTRAINT email_suppressions_source_event_id_fkey
        FOREIGN KEY (source_event_id)
        REFERENCES email_provider_events(id)
        ON DELETE SET NULL;

CREATE INDEX email_messages_terminal_retention_idx
    ON email_messages (terminal_at)
    WHERE terminal_at IS NOT NULL;

CREATE INDEX email_provider_events_retention_idx
    ON email_provider_events (processed_at)
    WHERE processed_at IS NOT NULL;
