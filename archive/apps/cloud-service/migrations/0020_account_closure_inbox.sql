CREATE TABLE cloud_account_closure_inbox (
  event_id UUID PRIMARY KEY,
  event_type TEXT NOT NULL,
  saga_id UUID NOT NULL,
  principal_id UUID NOT NULL,
  event_fingerprint BYTEA NOT NULL,
  processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX cloud_account_closure_inbox_principal
  ON cloud_account_closure_inbox (principal_id, processed_at DESC);
