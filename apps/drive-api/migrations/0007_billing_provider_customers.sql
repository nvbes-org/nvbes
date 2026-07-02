CREATE TABLE IF NOT EXISTS billing_provider_customers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  billing_account_id UUID NOT NULL REFERENCES billing_accounts (id) ON DELETE CASCADE,
  provider billing_provider NOT NULL,
  provider_customer_id TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (provider, provider_customer_id)
);

INSERT INTO billing_provider_customers (
  workspace_id,
  billing_account_id,
  provider,
  provider_customer_id,
  status,
  created_at,
  updated_at
)
SELECT workspace_id,
       id,
       'stripe',
       stripe_customer_id,
       'active',
       created_at,
       updated_at
FROM billing_accounts
WHERE stripe_customer_id IS NOT NULL
ON CONFLICT (provider, provider_customer_id) DO NOTHING;
