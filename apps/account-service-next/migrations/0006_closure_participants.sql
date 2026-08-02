CREATE TABLE account_closure_participants (
  saga_id UUID NOT NULL REFERENCES account_closure_sagas(id) ON DELETE CASCADE,
  participant TEXT NOT NULL
    CHECK (participant IN ('cloud', 'billing', 'identity', 'account')),
  ordinal SMALLINT NOT NULL CHECK (ordinal > 0),
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
  attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  last_error TEXT,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (saga_id, participant),
  UNIQUE (saga_id, ordinal)
);

CREATE INDEX account_closure_participants_resumable
  ON account_closure_participants (saga_id, ordinal)
  WHERE status <> 'completed';
