-- Durable, job-scoped delivery ledger for Account worker email effects.
--
-- SMTP remains an at-least-once transport: if the provider accepts a message
-- and the database commit then fails, the retry can send the same Message-ID
-- again. The stable identifiers below make that ambiguity observable and let
-- cooperating relays/providers deduplicate it.

ALTER TYPE email_message_status ADD VALUE IF NOT EXISTS 'sending';
ALTER TYPE email_message_status ADD VALUE IF NOT EXISTS 'failed';

ALTER TABLE email_messages
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN message_id TEXT,
    ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN last_attempt_at TIMESTAMPTZ,
    ADD COLUMN last_error_class VARCHAR(16),
    ADD COLUMN last_error_code VARCHAR(64),
    ADD COLUMN last_error_summary VARCHAR(256);

CREATE FUNCTION ensure_email_message_delivery_identity()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF NEW.message_id IS NULL THEN
        NEW.message_id :=
            '<account-job-' || NEW.job_id::text || '@worker.nvbes.fr>';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER email_messages_delivery_identity
BEFORE INSERT OR UPDATE OF job_id, message_id
ON email_messages
FOR EACH ROW
EXECUTE FUNCTION ensure_email_message_delivery_identity();

UPDATE email_messages
SET message_id = '<account-job-' || job_id::text || '@worker.nvbes.fr>'
WHERE message_id IS NULL;

ALTER TABLE email_messages
    ALTER COLUMN message_id SET NOT NULL,
    ALTER COLUMN sent_at DROP NOT NULL,
    ADD CONSTRAINT email_messages_attempt_count_non_negative
        CHECK (attempt_count >= 0),
    ADD CONSTRAINT email_messages_error_class_known
        CHECK (
            last_error_class IS NULL
            OR last_error_class IN ('transient', 'permanent')
        );

CREATE UNIQUE INDEX email_messages_job_id_unique
    ON email_messages (job_id);

CREATE UNIQUE INDEX email_messages_message_id_unique
    ON email_messages (message_id);
