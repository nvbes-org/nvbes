-- Email delivery lifecycle data is now owned exclusively by nvbes-email-worker.
-- Billing retains its business audit_events table, but no recipient-level
-- delivery ledger or suppression data.

DROP TABLE email_events;
DROP TABLE suppressed_emails;
DROP TABLE email_messages;

DROP TYPE email_event_type;
DROP TYPE email_message_status;
