-- migrate:up
CREATE SCHEMA IF NOT EXISTS consent;

CREATE TABLE consent.legal_agreements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_id UUID NOT NULL,
    document_type VARCHAR(50) NOT NULL,
    document_version VARCHAR(20) NOT NULL,
    agreed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    ip_hash VARCHAR(64) NOT NULL,
    user_agent TEXT
);

CREATE INDEX legal_agreements_subject_doc_idx
    ON consent.legal_agreements (subject_id, document_type, agreed_at DESC);

CREATE TABLE consent.cookie_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_id UUID NOT NULL UNIQUE,
    functional BOOLEAN NOT NULL DEFAULT true,
    analytics BOOLEAN NOT NULL DEFAULT false,
    marketing BOOLEAN NOT NULL DEFAULT false,
    gpc_signal BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX cookie_preferences_subject_idx
    ON consent.cookie_preferences (subject_id);

CREATE TABLE consent.audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_id UUID NOT NULL,
    action VARCHAR(50) NOT NULL,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX consent_audit_events_subject_idx
    ON consent.audit_events (subject_id, created_at DESC);

-- migrate:down
DROP TABLE IF EXISTS consent.audit_events;
DROP TABLE IF EXISTS consent.cookie_preferences;
DROP TABLE IF EXISTS consent.legal_agreements;
DROP SCHEMA IF EXISTS consent CASCADE;
