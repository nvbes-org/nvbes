CREATE TABLE IF NOT EXISTS idempotency_responses (
  idempotency_key TEXT NOT NULL,
  scope TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  response_status INT NOT NULL,
  response_body BYTEA NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '24 hours',
  PRIMARY KEY (idempotency_key, scope)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_responses_expires_at
  ON idempotency_responses (expires_at);
