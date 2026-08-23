-- Billing owns a minimal principal projection so its database is independently migratable.
CREATE TABLE IF NOT EXISTS principals (
  id UUID PRIMARY KEY,
  display_name TEXT,
  status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'suspended', 'deleted')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
