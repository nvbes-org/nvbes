CREATE TABLE access_review_reminders (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  campaign_id UUID NOT NULL REFERENCES access_review_campaigns(id) ON DELETE CASCADE,
  recipient_principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
  reminder_kind TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT access_review_reminder_kind CHECK (reminder_kind IN ('due_soon', 'overdue')),
  CONSTRAINT access_review_reminder_unique UNIQUE (
    campaign_id,
    recipient_principal_id,
    reminder_kind
  )
);

CREATE INDEX idx_access_review_reminders_tenant_campaign
  ON access_review_reminders (tenant_id, campaign_id, created_at DESC);

ALTER TABLE access_review_reminders ENABLE ROW LEVEL SECURITY;

CREATE POLICY access_review_reminder_isolation ON access_review_reminders
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
  WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
