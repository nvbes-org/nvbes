ALTER TABLE email_suppressions
    ADD COLUMN recipient_encryption_id UUID,
    ADD COLUMN reviewed_at TIMESTAMPTZ,
    ADD COLUMN reviewed_by VARCHAR(200),
    ADD COLUMN review_reason VARCHAR(500);

UPDATE email_suppressions AS suppression
SET recipient_encryption_id = message.id
FROM email_provider_events AS event
JOIN email_messages AS message
  ON message.provider_message_id = event.provider_message_id
WHERE suppression.source_event_id = event.id
  AND suppression.recipient_encryption_id IS NULL;

ALTER TABLE email_suppressions
    ALTER COLUMN recipient_encryption_id SET NOT NULL,
    ADD CONSTRAINT email_suppressions_review_audit CHECK (
        (reviewed_at IS NULL AND reviewed_by IS NULL AND review_reason IS NULL)
        OR (reviewed_at IS NOT NULL AND reviewed_by IS NOT NULL AND review_reason IS NOT NULL)
    );

CREATE TABLE email_operator_actions (
    id UUID PRIMARY KEY,
    action_kind VARCHAR(80) NOT NULL,
    actor VARCHAR(200) NOT NULL,
    reason VARCHAR(500) NOT NULL,
    message_id UUID REFERENCES email_messages(id) ON DELETE SET NULL,
    recipient_hash BYTEA CHECK (recipient_hash IS NULL OR octet_length(recipient_hash) = 32),
    outcome VARCHAR(80) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX email_operator_actions_created_idx
    ON email_operator_actions (created_at DESC);
