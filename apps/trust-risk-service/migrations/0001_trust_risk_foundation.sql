CREATE TABLE trust_risk_signals (
    id UUID PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    producer TEXT NOT NULL CHECK (length(producer) BETWEEN 3 AND 80),
    signal_kind TEXT NOT NULL CHECK (length(signal_kind) BETWEEN 3 AND 100),
    occurred_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    partition_key TEXT NOT NULL CHECK (length(partition_key) BETWEEN 3 AND 200),
    scope SMALLINT NOT NULL CHECK (scope IN (1, 2)),
    fingerprint BYTEA NOT NULL CHECK (octet_length(fingerprint) = 32),
    payload BYTEA NOT NULL CHECK (octet_length(payload) <= 196608),
    projected_at TIMESTAMPTZ,
    projection_attempts INTEGER NOT NULL DEFAULT 0 CHECK (projection_attempts >= 0),
    next_projection_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    lease_owner UUID,
    lease_expires_at TIMESTAMPTZ,
    quarantined_at TIMESTAMPTZ,
    retention_deadline TIMESTAMPTZ NOT NULL,
    legal_hold BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX trust_risk_signals_projection_claim_idx
    ON trust_risk_signals (next_projection_at, accepted_at)
    WHERE projected_at IS NULL AND quarantined_at IS NULL;
CREATE INDEX trust_risk_signals_partition_event_idx
    ON trust_risk_signals (partition_key, occurred_at, id);
CREATE INDEX trust_risk_signals_retention_idx
    ON trust_risk_signals (retention_deadline)
    WHERE NOT legal_hold;

CREATE TABLE trust_risk_signal_subjects (
    signal_id UUID NOT NULL REFERENCES trust_risk_signals(id) ON DELETE CASCADE,
    ordinal SMALLINT NOT NULL CHECK (ordinal >= 0),
    kind SMALLINT NOT NULL CHECK (kind BETWEEN 1 AND 6),
    namespace TEXT NOT NULL CHECK (length(namespace) BETWEEN 3 AND 80),
    opaque_id TEXT NOT NULL CHECK (length(opaque_id) BETWEEN 8 AND 200 AND position('@' IN opaque_id) = 0),
    scope SMALLINT NOT NULL CHECK (scope IN (1, 2)),
    tenant_id UUID,
    PRIMARY KEY (signal_id, ordinal),
    UNIQUE (signal_id, kind, namespace, opaque_id)
);
CREATE INDEX trust_risk_signal_subject_lookup_idx
    ON trust_risk_signal_subjects (kind, namespace, opaque_id, signal_id);

CREATE TABLE trust_risk_outbox (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    aggregate_id UUID NOT NULL,
    event_kind TEXT NOT NULL,
    partition_key TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    published_at TIMESTAMPTZ,
    UNIQUE (aggregate_id, event_kind)
);

CREATE TABLE trust_risk_feature_state (
    subject_kind SMALLINT NOT NULL,
    namespace TEXT NOT NULL,
    opaque_id TEXT NOT NULL,
    feature_version TEXT NOT NULL,
    features JSONB NOT NULL,
    event_watermark TIMESTAMPTZ NOT NULL,
    rebuilt_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (subject_kind, namespace, opaque_id, feature_version)
);

CREATE TABLE trust_risk_rule_sets (
    version TEXT PRIMARY KEY,
    feature_version TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('staged', 'active', 'retired')),
    canonical_json JSONB NOT NULL,
    checksum BYTEA NOT NULL CHECK (octet_length(checksum) = 32),
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    activated_by TEXT,
    activated_at TIMESTAMPTZ,
    CHECK ((state = 'active') = (activated_at IS NOT NULL))
);
CREATE UNIQUE INDEX trust_risk_one_active_rule_set_idx
    ON trust_risk_rule_sets ((state)) WHERE state = 'active';

CREATE TABLE trust_risk_evaluations (
    id UUID PRIMARY KEY,
    producer TEXT NOT NULL,
    assessment_key TEXT NOT NULL,
    operation_class TEXT NOT NULL,
    request_fingerprint BYTEA NOT NULL CHECK (octet_length(request_fingerprint) = 32),
    score SMALLINT NOT NULL CHECK (score BETWEEN 0 AND 100),
    band TEXT NOT NULL CHECK (band IN ('low', 'elevated', 'high', 'critical')),
    recommendation TEXT NOT NULL CHECK (recommendation IN ('allow', 'challenge', 'review', 'deny')),
    feature_version TEXT NOT NULL,
    rule_set_version TEXT NOT NULL REFERENCES trust_risk_rule_sets(version),
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    UNIQUE (producer, assessment_key)
);
CREATE INDEX trust_risk_evaluations_retention_idx ON trust_risk_evaluations (expires_at);

CREATE TABLE trust_risk_evaluation_features (
    evaluation_id UUID NOT NULL REFERENCES trust_risk_evaluations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL CHECK (
        value NOT IN ('NaN'::DOUBLE PRECISION, 'Infinity'::DOUBLE PRECISION, '-Infinity'::DOUBLE PRECISION)
    ),
    PRIMARY KEY (evaluation_id, name)
);

CREATE TABLE trust_risk_evaluation_reasons (
    evaluation_id UUID NOT NULL REFERENCES trust_risk_evaluations(id) ON DELETE CASCADE,
    ordinal SMALLINT NOT NULL CHECK (ordinal >= 0),
    code TEXT NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (evaluation_id, ordinal),
    UNIQUE (evaluation_id, code)
);

CREATE TABLE trust_risk_labels (
    id UUID PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    producer TEXT NOT NULL,
    evaluation_id UUID NOT NULL REFERENCES trust_risk_evaluations(id),
    review_case_id UUID,
    kind SMALLINT NOT NULL CHECK (kind BETWEEN 1 AND 6),
    source_class SMALLINT NOT NULL CHECK (source_class BETWEEN 1 AND 4),
    source_id TEXT NOT NULL,
    confidence DOUBLE PRECISION NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    actor TEXT,
    knowledge_at TIMESTAMPTZ NOT NULL,
    evidence_reference TEXT,
    mapping_version TEXT NOT NULL,
    corrects_label_id UUID REFERENCES trust_risk_labels(id),
    fingerprint BYTEA NOT NULL CHECK (octet_length(fingerprint) = 32),
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    CHECK (corrects_label_id IS NULL OR corrects_label_id <> id)
);

CREATE TABLE trust_risk_canonical_labels (
    evaluation_id UUID PRIMARY KEY REFERENCES trust_risk_evaluations(id) ON DELETE CASCADE,
    label_id UUID NOT NULL UNIQUE REFERENCES trust_risk_labels(id),
    resolved_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE trust_risk_review_cases (
    id UUID PRIMARY KEY,
    evaluation_id UUID NOT NULL UNIQUE REFERENCES trust_risk_evaluations(id),
    state TEXT NOT NULL CHECK (state IN ('open', 'in_review', 'resolved', 'inconclusive')),
    assigned_to TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL
);
ALTER TABLE trust_risk_labels
    ADD CONSTRAINT trust_risk_labels_review_case_fk
    FOREIGN KEY (review_case_id) REFERENCES trust_risk_review_cases(id);

CREATE TABLE trust_risk_review_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    review_case_id UUID NOT NULL REFERENCES trust_risk_review_cases(id) ON DELETE CASCADE,
    from_state TEXT,
    to_state TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE trust_risk_audit_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    legal_hold BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE trust_risk_projection_quarantine (
    signal_id UUID PRIMARY KEY REFERENCES trust_risk_signals(id) ON DELETE CASCADE,
    error_code TEXT NOT NULL,
    attempts INTEGER NOT NULL CHECK (attempts > 0),
    quarantined_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    retry_requested_at TIMESTAMPTZ
);

CREATE TABLE trust_risk_rebuild_requests (
    id UUID PRIMARY KEY,
    subject_kind SMALLINT NOT NULL,
    namespace TEXT NOT NULL,
    opaque_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    completed_at TIMESTAMPTZ
);

INSERT INTO trust_risk_rule_sets (
    version, feature_version, state, canonical_json, checksum, created_by, activated_by, activated_at
) VALUES (
    'baseline-v1',
    'features-v1',
    'active',
    '{"version":"baseline-v1","feature_version":"features-v1","thresholds":{"challenge":30,"review":60,"deny":85},"rules":[{"code":"high-network-risk","predicate":{"op":"gte","feature":"network_risk_score_max_1h","value":75},"score_delta":35,"reasons":["network.high_risk"],"minimum_recommendation":null},{"code":"automation-confidence","predicate":{"op":"gte","feature":"automation_confidence_max_1h","value":0.8},"score_delta":30,"reasons":["automation.high_confidence"],"minimum_recommendation":"challenge"},{"code":"high-velocity","predicate":{"op":"gte","feature":"events_1h","value":10},"score_delta":25,"reasons":["velocity.high"],"minimum_recommendation":null},{"code":"principal-reuse","predicate":{"op":"gte","feature":"linked_principals_24h","value":5},"score_delta":20,"reasons":["subject.reuse"],"minimum_recommendation":null},{"code":"negative-label","predicate":{"op":"gte","feature":"negative_labels","value":1},"score_delta":45,"reasons":["label.negative"],"minimum_recommendation":"review"},{"code":"positive-reputation","predicate":{"op":"gte","feature":"positive_labels","value":1},"score_delta":-15,"reasons":["reputation.positive"],"minimum_recommendation":null}]}'::jsonb,
    decode('8e0bc49703587341235151d23e216ea635251c37305ed55ef920ddebbe3ce2bc', 'hex'),
    'migration',
    'migration',
    clock_timestamp()
);
