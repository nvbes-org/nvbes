CREATE TYPE email_category AS ENUM (
    'credential',
    'account_security',
    'billing',
    'reminder'
);

CREATE TYPE email_message_state AS ENUM (
    'accepted',
    'dispatching',
    'provider_accepted',
    'delivered',
    'deferred',
    'hard_bounced',
    'complained',
    'unsubscribed',
    'suppressed',
    'expired',
    'dropped',
    'failed'
);

CREATE TYPE email_attempt_outcome AS ENUM (
    'started',
    'provider_accepted',
    'transient_failure',
    'permanent_failure',
    'ambiguous_failure',
    'expired'
);

CREATE TABLE email_messages (
    id UUID PRIMARY KEY,
    producer VARCHAR(64) NOT NULL,
    idempotency_key VARCHAR(200) NOT NULL,
    command_fingerprint BYTEA NOT NULL CHECK (octet_length(command_fingerprint) = 32),
    request_id VARCHAR(200) NOT NULL,
    correlation_id VARCHAR(200) NOT NULL,
    category email_category NOT NULL,
    template_name VARCHAR(80) NOT NULL,
    template_version SMALLINT NOT NULL CHECK (template_version > 0),
    recipient_hash BYTEA NOT NULL CHECK (octet_length(recipient_hash) = 32),
    recipient_ciphertext BYTEA NOT NULL,
    recipient_nonce BYTEA NOT NULL CHECK (octet_length(recipient_nonce) = 12),
    template_ciphertext BYTEA NOT NULL,
    template_nonce BYTEA NOT NULL CHECK (octet_length(template_nonce) = 12),
    deliver_before TIMESTAMPTZ NOT NULL,
    state email_message_state NOT NULL DEFAULT 'accepted',
    message_id VARCHAR(255) NOT NULL,
    provider_message_id VARCHAR(255),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    lease_token UUID,
    lease_expires_at TIMESTAMPTZ,
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    provider_accepted_at TIMESTAMPTZ,
    terminal_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT email_messages_idempotency UNIQUE (producer, idempotency_key),
    CONSTRAINT email_messages_message_id_unique UNIQUE (message_id),
    CONSTRAINT email_messages_deadline_after_acceptance CHECK (deliver_before > accepted_at)
);

CREATE INDEX email_messages_due_idx
    ON email_messages (next_attempt_at, accepted_at)
    WHERE state IN ('accepted', 'deferred');
CREATE INDEX email_messages_provider_id_idx
    ON email_messages (provider_message_id)
    WHERE provider_message_id IS NOT NULL;
CREATE INDEX email_messages_recipient_hash_idx ON email_messages (recipient_hash);

CREATE TABLE email_delivery_attempts (
    id UUID PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES email_messages(id) ON DELETE CASCADE,
    attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
    lease_token UUID NOT NULL,
    outcome email_attempt_outcome NOT NULL DEFAULT 'started',
    provider_message_id VARCHAR(255),
    failure_code VARCHAR(80),
    started_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    finished_at TIMESTAMPTZ,
    CONSTRAINT email_delivery_attempt_number UNIQUE (message_id, attempt_number),
    CONSTRAINT email_delivery_attempt_lease UNIQUE (lease_token)
);

CREATE TABLE email_provider_events (
    id UUID PRIMARY KEY,
    provider VARCHAR(40) NOT NULL,
    provider_event_id VARCHAR(255) NOT NULL,
    sns_message_id VARCHAR(255) NOT NULL,
    provider_message_id VARCHAR(255),
    event_type VARCHAR(80) NOT NULL,
    diagnostic JSONB NOT NULL DEFAULT '{}'::jsonb,
    received_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    processed_at TIMESTAMPTZ,
    processing_result VARCHAR(80),
    CONSTRAINT email_provider_event_unique UNIQUE (provider, provider_event_id),
    CONSTRAINT email_provider_sns_message_unique UNIQUE (sns_message_id)
);

CREATE TABLE email_suppressions (
    recipient_hash BYTEA PRIMARY KEY CHECK (octet_length(recipient_hash) = 32),
    recipient_ciphertext BYTEA NOT NULL,
    recipient_nonce BYTEA NOT NULL CHECK (octet_length(recipient_nonce) = 12),
    scope VARCHAR(40) NOT NULL CHECK (scope IN ('all', 'optional')),
    reason VARCHAR(80) NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    source_event_id UUID REFERENCES email_provider_events(id),
    soft_bounce_count INTEGER NOT NULL DEFAULT 0 CHECK (soft_bounce_count >= 0),
    suppressed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    released_at TIMESTAMPTZ,
    released_by VARCHAR(200),
    release_reason VARCHAR(500),
    CONSTRAINT email_suppressions_release_audit CHECK (
        (active AND released_at IS NULL AND released_by IS NULL AND release_reason IS NULL)
        OR (NOT active AND released_at IS NOT NULL AND released_by IS NOT NULL AND release_reason IS NOT NULL)
        OR (NOT active AND reason = 'soft_bounce_threshold' AND released_at IS NULL)
    )
);
