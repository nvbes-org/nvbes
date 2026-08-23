-- Billing worker email and audit persistence.

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'email_event_type') THEN
    CREATE TYPE email_event_type AS ENUM (
      'email_delivered',
      'email_bounced',
      'email_complained',
      'email_dropped',
      'email_blacklisted',
      'email_unsubscribed',
      'email_opened',
      'email_clicked',
      'email_sent'
    );
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'email_message_status') THEN
    CREATE TYPE email_message_status AS ENUM (
      'queued',
      'sent',
      'delivered',
      'bounced',
      'complained',
      'dropped'
    );
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS audit_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  actor_principal_id UUID REFERENCES users(principal_id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  target_type TEXT NOT NULL,
  target_id UUID,
  ip INET,
  user_agent TEXT,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  previous_event_hash TEXT,
  event_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS email_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  provider_event_id TEXT NOT NULL,
  provider_email_id TEXT,
  email TEXT NOT NULL,
  event_type email_event_type NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL,
  details JSONB DEFAULT '{}'::jsonb,
  processed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS suppressed_emails (
  email TEXT PRIMARY KEY,
  reason TEXT NOT NULL,
  suppressed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  details JSONB DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS email_messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  job_id UUID NOT NULL,
  business_type TEXT NOT NULL,
  recipient_email TEXT NOT NULL,
  recipient_hash TEXT NOT NULL,
  provider_email_id TEXT,
  status email_message_status NOT NULL DEFAULT 'queued',
  sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_email_events_provider_event_id
  ON email_events(provider_event_id);
CREATE INDEX IF NOT EXISTS idx_email_events_provider_email_id
  ON email_events(provider_email_id);
CREATE INDEX IF NOT EXISTS idx_email_messages_provider_email_id
  ON email_messages(provider_email_id);
