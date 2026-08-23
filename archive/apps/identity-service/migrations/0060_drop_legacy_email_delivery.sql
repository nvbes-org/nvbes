-- Email delivery lifecycle data is now owned exclusively by nvbes-email-worker.
-- These historical tables contained recipient PII in clear text and must not
-- survive the cutover once all operational and privacy consumers use the
-- authenticated email operations contract.

DROP TABLE email_events;
DROP TABLE suppressed_emails;
DROP TABLE email_messages;

DROP FUNCTION ensure_email_message_delivery_identity();
DROP TYPE email_event_type;
DROP TYPE email_message_status;
