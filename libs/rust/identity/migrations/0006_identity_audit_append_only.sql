CREATE FUNCTION identity_reject_audit_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'identity audit events are append-only';
END;
$$;

CREATE TRIGGER identity_audit_events_append_only
BEFORE UPDATE OR DELETE ON identity_audit_events
FOR EACH ROW EXECUTE FUNCTION identity_reject_audit_mutation();
