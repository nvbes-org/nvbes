DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workspace_status') THEN
        CREATE TYPE workspace_status AS ENUM ('active', 'suspended', 'deleted');
    END IF;
END $$;

ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS status workspace_status NOT NULL DEFAULT 'active';

CREATE INDEX IF NOT EXISTS idx_workspaces_status ON workspaces(status);
