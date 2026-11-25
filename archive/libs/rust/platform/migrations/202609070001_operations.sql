-- Dedicated Platform Operations database. No foreign tables or domain credentials.
CREATE TABLE operations_cases (
    id UUID PRIMARY KEY,
    version BIGINT NOT NULL CHECK (version > 0),
    document JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE operations_costs (
    id UUID PRIMARY KEY,
    month DATE NOT NULL CHECK (extract(day FROM month) = 1),
    document JSONB NOT NULL
);
CREATE INDEX operations_costs_month ON operations_costs(month, id);
CREATE UNIQUE INDEX operations_costs_one_correction ON operations_costs ((document->>'replaces'))
    WHERE document->>'replaces' IS NOT NULL;
CREATE TABLE operations_audit (
    sequence BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    actor TEXT NOT NULL,
    idempotency_key UUID NOT NULL,
    case_id UUID REFERENCES operations_cases(id),
    request JSONB NOT NULL,
    receipt JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (actor, idempotency_key)
);
CREATE INDEX operations_audit_case ON operations_audit(case_id, sequence);
CREATE FUNCTION operations_reject_audit_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'operations audit is append-only';
END;
$$;
CREATE TRIGGER operations_audit_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON operations_audit
    FOR EACH STATEMENT EXECUTE FUNCTION operations_reject_audit_mutation();
CREATE TRIGGER operations_costs_immutable
    BEFORE UPDATE OR DELETE OR TRUNCATE ON operations_costs
    FOR EACH STATEMENT EXECUTE FUNCTION operations_reject_audit_mutation();
