TRUNCATE TABLE account_privacy_exports;

ALTER TABLE account_privacy_exports
  ALTER COLUMN document DROP NOT NULL,
  ALTER COLUMN completed_at DROP NOT NULL,
  ALTER COLUMN expires_at DROP NOT NULL,
  ADD COLUMN status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
  ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ADD COLUMN last_error TEXT,
  ADD CONSTRAINT account_privacy_exports_completion_valid CHECK (
    (status = 'completed' AND document IS NOT NULL AND completed_at IS NOT NULL AND expires_at IS NOT NULL)
    OR status <> 'completed'
  );

CREATE UNIQUE INDEX account_privacy_exports_active_unique
  ON account_privacy_exports (principal_id)
  WHERE status IN ('pending', 'processing');

CREATE TABLE account_export_participants (
  export_id UUID NOT NULL REFERENCES account_privacy_exports(id) ON DELETE CASCADE,
  participant TEXT NOT NULL
    CHECK (participant IN ('cloud', 'billing', 'identity', 'account')),
  ordinal SMALLINT NOT NULL CHECK (ordinal > 0),
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
  attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
  fragment JSONB,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  last_error TEXT,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (export_id, participant),
  UNIQUE (export_id, ordinal),
  CHECK ((status = 'completed' AND fragment IS NOT NULL) OR status <> 'completed')
);

CREATE INDEX account_export_participants_resumable
  ON account_export_participants (export_id, ordinal)
  WHERE status <> 'completed';
