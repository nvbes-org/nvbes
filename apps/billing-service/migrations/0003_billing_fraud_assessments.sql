-- Billing fraud assessments are owned by billing-service and feed PSP routing decisions.

CREATE TABLE IF NOT EXISTS billing_fraud_assessments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  actor_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  provider billing_provider,
  checkout_id TEXT,
  payment_id TEXT,
  plan_code TEXT NOT NULL,
  amount_minor BIGINT NOT NULL CHECK (amount_minor >= 0),
  currency CHAR(3) NOT NULL CHECK (currency = upper(currency)),
  ip_address INET,
  geo_country_code CHAR(2) CHECK (geo_country_code IS NULL OR geo_country_code = upper(geo_country_code)),
  billing_country_code CHAR(2) CHECK (billing_country_code IS NULL OR billing_country_code = upper(billing_country_code)),
  network_kind TEXT NOT NULL,
  network_risk_score SMALLINT NOT NULL CHECK (network_risk_score BETWEEN 0 AND 100),
  network_risk_labels TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
  fraud_score SMALLINT NOT NULL CHECK (fraud_score BETWEEN 0 AND 100),
  fraud_decision TEXT NOT NULL,
  network_threat TEXT NOT NULL,
  fraud_labels TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
  fraud_reasons TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
  enforcement_action TEXT NOT NULL,
  review_status TEXT NOT NULL DEFAULT 'pending',
  reviewed_at TIMESTAMPTZ,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_billing_fraud_assessments_workspace_created
  ON billing_fraud_assessments (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_billing_fraud_assessments_ip_created
  ON billing_fraud_assessments (ip_address, created_at DESC)
  WHERE ip_address IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_billing_fraud_assessments_decision_created
  ON billing_fraud_assessments (fraud_decision, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_billing_fraud_assessments_review_created
  ON billing_fraud_assessments (review_status, created_at DESC);
