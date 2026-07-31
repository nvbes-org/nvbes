CREATE TABLE account_profiles (
  principal_id UUID PRIMARY KEY,
  firstname TEXT CHECK (firstname IS NULL OR char_length(firstname) <= 100),
  lastname TEXT CHECK (lastname IS NULL OR char_length(lastname) <= 100),
  username TEXT CHECK (username IS NULL OR char_length(username) <= 100),
  birthdate DATE,
  region TEXT CHECK (region IS NULL OR char_length(region) <= 64),
  avatar_object_key TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX account_profiles_username_unique
  ON account_profiles (lower(username))
  WHERE username IS NOT NULL;

CREATE TABLE account_preferences (
  principal_id UUID PRIMARY KEY,
  theme TEXT NOT NULL DEFAULT 'system'
    CHECK (theme IN ('system', 'light', 'dark')),
  language TEXT NOT NULL DEFAULT 'fr'
    CHECK (language IN ('fr', 'en')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE account_notifications (
  principal_id UUID PRIMARY KEY,
  email BOOLEAN NOT NULL DEFAULT TRUE,
  push BOOLEAN NOT NULL DEFAULT TRUE,
  in_app BOOLEAN NOT NULL DEFAULT TRUE,
  marketing_email BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE account_consents (
  id UUID PRIMARY KEY,
  principal_id UUID NOT NULL,
  consent_type TEXT NOT NULL CHECK (char_length(consent_type) BETWEEN 1 AND 100),
  document_version TEXT NOT NULL CHECK (char_length(document_version) BETWEEN 1 AND 100),
  ip_address INET,
  granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  revoked_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX account_consents_active_unique
  ON account_consents (principal_id, consent_type, document_version)
  WHERE revoked_at IS NULL;

CREATE INDEX account_consents_principal_history
  ON account_consents (principal_id, granted_at DESC, id DESC);

CREATE TABLE account_privacy_exports (
  id UUID PRIMARY KEY,
  principal_id UUID NOT NULL,
  document JSONB NOT NULL,
  requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX account_privacy_exports_principal_latest
  ON account_privacy_exports (principal_id, completed_at DESC);

CREATE TABLE account_closure_sagas (
  id UUID PRIMARY KEY,
  principal_id UUID NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending', 'dispatching', 'completed', 'failed', 'cancelled')),
  requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  completed_at TIMESTAMPTZ,
  last_error TEXT
);

CREATE UNIQUE INDEX account_closure_sagas_active_unique
  ON account_closure_sagas (principal_id)
  WHERE status IN ('pending', 'dispatching');

CREATE TABLE account_outbox_events (
  id UUID PRIMARY KEY,
  aggregate_type TEXT NOT NULL,
  aggregate_id UUID NOT NULL,
  event_type TEXT NOT NULL,
  payload JSONB NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  published_at TIMESTAMPTZ,
  publish_attempts INTEGER NOT NULL DEFAULT 0
    CHECK (publish_attempts >= 0)
);

CREATE INDEX account_outbox_events_unpublished
  ON account_outbox_events (occurred_at, id)
  WHERE published_at IS NULL;

