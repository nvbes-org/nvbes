CREATE TYPE access_review_campaign_status AS ENUM ('draft', 'active', 'closed');
CREATE TYPE access_review_item_type AS ENUM ('member', 'role', 'service_account', 'oauth_client');
CREATE TYPE access_review_item_decision AS ENUM ('pending', 'approved', 'revoked', 'changed');

CREATE TABLE access_review_campaigns (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  description TEXT,
  status access_review_campaign_status NOT NULL DEFAULT 'active',
  starts_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  due_at TIMESTAMPTZ NOT NULL,
  created_by UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  closed_at TIMESTAMPTZ,
  CONSTRAINT access_review_campaign_due_after_start CHECK (due_at > starts_at)
);

CREATE TABLE access_review_items (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  campaign_id UUID NOT NULL REFERENCES access_review_campaigns(id) ON DELETE CASCADE,
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  item_type access_review_item_type NOT NULL,
  subject_id TEXT NOT NULL,
  subject_label TEXT NOT NULL,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  role TEXT,
  status TEXT NOT NULL,
  evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
  decision access_review_item_decision NOT NULL DEFAULT 'pending',
  reviewed_by UUID REFERENCES principals(id) ON DELETE SET NULL,
  reviewed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_access_review_campaigns_tenant_status
  ON access_review_campaigns (tenant_id, status, due_at);

CREATE INDEX idx_access_review_items_campaign
  ON access_review_items (campaign_id, item_type, decision);

CREATE UNIQUE INDEX idx_access_review_items_campaign_subject
  ON access_review_items (
    campaign_id,
    tenant_id,
    item_type,
    subject_id,
    COALESCE(workspace_id, '00000000-0000-0000-0000-000000000000'::uuid)
  );

ALTER TABLE access_review_campaigns ENABLE ROW LEVEL SECURITY;
ALTER TABLE access_review_items ENABLE ROW LEVEL SECURITY;

CREATE POLICY access_review_campaign_isolation ON access_review_campaigns
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
  WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

CREATE POLICY access_review_item_isolation ON access_review_items
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
  WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
