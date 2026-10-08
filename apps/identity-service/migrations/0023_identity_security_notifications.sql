CREATE TABLE identity_security_notifications (
    id UUID PRIMARY KEY,
    event_id UUID NOT NULL REFERENCES identity_outbox(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES identity_principals(id) ON DELETE CASCADE,
    recipient_identifier_id UUID,
    command JSONB,
    state TEXT NOT NULL CHECK(state IN ('pending','sending','accepted','failed','expired','no_recipient','no_snapshot')),
    attempts SMALLINT NOT NULL DEFAULT 0 CHECK(attempts BETWEEN 0 AND 8),
    available_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    deliver_before TIMESTAMPTZ NOT NULL,
    lease_token UUID,
    lease_expires_at TIMESTAMPTZ,
    receipt_id TEXT,
    email_accepted_at TIMESTAMPTZ,
    outcome TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    settled_at TIMESTAMPTZ,
    UNIQUE(event_id, recipient_identifier_id),
    CHECK ((state IN ('pending','sending')) = (command IS NOT NULL)),
    CHECK ((state='sending') = (lease_token IS NOT NULL AND lease_expires_at IS NOT NULL)),
    CHECK (command IS NULL OR jsonb_typeof(command)='object')
);
CREATE INDEX identity_security_notifications_due_idx
    ON identity_security_notifications(available_at, created_at)
    WHERE state IN ('pending','sending');

-- Old events have no trustworthy historical recipient snapshot. Surface them
-- for operator review instead of sending to an address added after the event.
INSERT INTO identity_security_notifications(id,event_id,principal_id,state,deliver_before,outcome,settled_at)
SELECT o.id,o.id,o.aggregate_id,'no_snapshot',o.occurred_at+interval '24 hours','historical_recipient_unknown',clock_timestamp()
FROM identity_outbox o JOIN identity_principals p ON p.id=o.aggregate_id
WHERE o.event_type IN ('identity.mfa_recovery_codes_generated','identity.mfa_recovery_started','identity.mfa_recovered','identity.mfa_recovery_cancelled');
