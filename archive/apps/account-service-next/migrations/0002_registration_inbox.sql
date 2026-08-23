CREATE TABLE account_inbox_events (
  event_id UUID PRIMARY KEY,
  event_type TEXT NOT NULL,
  principal_id UUID NOT NULL,
  processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX account_inbox_events_principal
  ON account_inbox_events (principal_id, processed_at DESC);
