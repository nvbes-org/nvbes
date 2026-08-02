INSERT INTO principals (id, display_name)
SELECT principal_id, name
FROM users
ON CONFLICT (id) DO NOTHING;

ALTER TABLE users
  ADD CONSTRAINT users_principal_projection_fk
  FOREIGN KEY (principal_id) REFERENCES principals(id) ON DELETE CASCADE;

CREATE TABLE billing_account_closure_inbox (
  event_id UUID PRIMARY KEY,
  event_type TEXT NOT NULL,
  saga_id UUID NOT NULL,
  principal_id UUID NOT NULL,
  event_fingerprint BYTEA NOT NULL,
  processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX billing_account_closure_inbox_principal
  ON billing_account_closure_inbox (principal_id, processed_at DESC);
