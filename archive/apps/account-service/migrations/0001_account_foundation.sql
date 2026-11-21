CREATE TABLE account_profiles (
    principal_id UUID PRIMARY KEY,
    firstname TEXT,
    lastname TEXT,
    username TEXT UNIQUE,
    birthdate DATE,
    region TEXT,
    lifecycle_status TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_status IN ('active', 'closure_pending', 'closed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    closed_at TIMESTAMPTZ,
    CHECK (username IS NULL OR (char_length(username) BETWEEN 3 AND 100 AND username = lower(username))),
    CHECK (region IS NULL OR char_length(region) BETWEEN 2 AND 32),
    CHECK ((lifecycle_status = 'closed') = (closed_at IS NOT NULL))
);

CREATE TABLE account_preferences (
    principal_id UUID PRIMARY KEY REFERENCES account_profiles(principal_id) ON DELETE CASCADE,
    theme TEXT NOT NULL DEFAULT 'system' CHECK (theme IN ('system', 'light', 'dark')),
    language TEXT NOT NULL DEFAULT 'fr' CHECK (language IN ('fr', 'en')),
    email_notifications BOOLEAN NOT NULL DEFAULT true,
    push_notifications BOOLEAN NOT NULL DEFAULT true,
    in_app_notifications BOOLEAN NOT NULL DEFAULT true,
    marketing_email BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE account_teams (
    id UUID PRIMARY KEY,
    owner_principal_id UUID NOT NULL REFERENCES account_profiles(principal_id),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    join_code_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(join_code_hash) = 32),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'closed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX account_teams_owner_idx ON account_teams(owner_principal_id, created_at DESC);

CREATE TABLE account_team_memberships (
    team_id UUID NOT NULL REFERENCES account_teams(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES account_profiles(principal_id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'member')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (team_id, principal_id)
);

CREATE UNIQUE INDEX account_team_single_owner_idx
    ON account_team_memberships(team_id) WHERE role = 'owner';
CREATE INDEX account_team_memberships_principal_idx
    ON account_team_memberships(principal_id, created_at DESC);

CREATE TABLE account_exports (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES account_profiles(principal_id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'expired')),
    document JSONB,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    completed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    last_error TEXT,
    CHECK (document IS NULL OR jsonb_typeof(document) = 'object'),
    CHECK ((status = 'completed') = (document IS NOT NULL AND completed_at IS NOT NULL AND expires_at IS NOT NULL))
);

CREATE UNIQUE INDEX account_exports_active_principal_idx
    ON account_exports(principal_id) WHERE status IN ('pending', 'processing');
CREATE INDEX account_exports_principal_time_idx
    ON account_exports(principal_id, requested_at DESC);

CREATE TABLE account_closures (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES account_profiles(principal_id),
    status TEXT NOT NULL CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled')),
    execute_after TIMESTAMPTZ NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    completed_at TIMESTAMPTZ,
    last_error TEXT,
    CHECK (execute_after > requested_at),
    CHECK ((status = 'completed') = (completed_at IS NOT NULL))
);

CREATE UNIQUE INDEX account_closures_active_principal_idx
    ON account_closures(principal_id) WHERE status IN ('pending', 'processing');
CREATE INDEX account_closures_due_idx ON account_closures(execute_after) WHERE status = 'pending';

CREATE TABLE account_audit_events (
    id UUID PRIMARY KEY,
    principal_id UUID,
    actor_principal_id UUID,
    event_type TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id UUID,
    correlation_id UUID NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    details JSONB NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(details) = 'object')
);

CREATE INDEX account_audit_principal_time_idx
    ON account_audit_events(principal_id, occurred_at DESC);
CREATE INDEX account_audit_resource_time_idx
    ON account_audit_events(resource_type, resource_id, occurred_at DESC);

CREATE FUNCTION account_reject_audit_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'account audit events are append-only';
END;
$$;

CREATE TRIGGER account_audit_events_append_only
BEFORE UPDATE OR DELETE ON account_audit_events
FOR EACH ROW EXECUTE FUNCTION account_reject_audit_mutation();

CREATE TABLE account_outbox (
    id UUID PRIMARY KEY,
    event_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    payload JSONB NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    published_at TIMESTAMPTZ,
    attempts SMALLINT NOT NULL DEFAULT 0 CHECK (attempts >= 0)
);

CREATE INDEX account_outbox_pending_idx ON account_outbox(occurred_at) WHERE published_at IS NULL;
