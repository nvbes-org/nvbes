CREATE TABLE access_review_schedules (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  description TEXT,
  include_members BOOLEAN NOT NULL,
  include_roles BOOLEAN NOT NULL,
  include_service_accounts BOOLEAN NOT NULL,
  include_oauth_clients BOOLEAN NOT NULL,
  recurrence_days INTEGER NOT NULL,
  due_after_days INTEGER NOT NULL,
  next_run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_campaign_id UUID REFERENCES access_review_campaigns(id) ON DELETE SET NULL,
  created_by UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  disabled_at TIMESTAMPTZ,
  CONSTRAINT access_review_schedule_recurrence_bounds
    CHECK (recurrence_days BETWEEN 7 AND 366),
  CONSTRAINT access_review_schedule_due_after_bounds
    CHECK (due_after_days BETWEEN 1 AND recurrence_days),
  CONSTRAINT access_review_schedule_non_empty_scope
    CHECK (
      include_members
      OR include_roles
      OR include_service_accounts
      OR include_oauth_clients
    )
);

CREATE INDEX idx_access_review_schedules_due
  ON access_review_schedules (tenant_id, disabled_at, next_run_at);

ALTER TABLE access_review_schedules ENABLE ROW LEVEL SECURITY;

CREATE POLICY access_review_schedule_isolation ON access_review_schedules
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
  WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
