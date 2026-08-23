-- Store immutable external anchors for audit chains.
CREATE TABLE audit_external_anchors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    snapshot_digest TEXT NOT NULL UNIQUE,
    signature TEXT NOT NULL,
    signing_key_id TEXT NOT NULL,
    object_key TEXT NOT NULL UNIQUE,
    tenant_chain_count BIGINT NOT NULL CHECK (tenant_chain_count >= 0),
    event_count BIGINT NOT NULL CHECK (event_count >= 0),
    anchored_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_external_anchors_anchored_at
    ON audit_external_anchors (anchored_at DESC);

CREATE OR REPLACE FUNCTION audit_external_anchors_prevent_mutation()
RETURNS TRIGGER AS $$
BEGIN
  RAISE EXCEPTION 'audit_external_anchors is append-only and immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_external_anchors_prevent_update
BEFORE UPDATE ON audit_external_anchors
FOR EACH ROW
EXECUTE FUNCTION audit_external_anchors_prevent_mutation();

CREATE TRIGGER trg_audit_external_anchors_prevent_delete
BEFORE DELETE ON audit_external_anchors
FOR EACH ROW
EXECUTE FUNCTION audit_external_anchors_prevent_mutation();
