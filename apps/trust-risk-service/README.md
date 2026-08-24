# nvbes Trust/Risk Service

Independent runtime for cross-product fraud, abuse, bot and risk intelligence.
It accepts versioned signals, derives replayable features and returns explainable
risk recommendations. Products retain their business policy and enforcement.

The V0 implementation uses its own PostgreSQL database. It does not restore an
archived Billing application and does not depend on Backoffice for readiness.

## Runtime

The service exposes the versioned `nvbes.trust_risk.v1` signal, assessment,
label and operations APIs over gRPC on the same listener as `/health/live`,
`/health/ready` and authenticated `/metrics`.

```bash
cargo run -p nvbes-trust-risk-service -- migrate
cargo run -p nvbes-trust-risk-service
```

Normal startup never runs migrations. Producers receive individual tokens and
explicit signal-family, assessment and label permissions through
`NVBES_TRUST_RISK_PRODUCER_POLICIES`. Operators use a separate policy set and
cannot activate a rule set they staged themselves.

PostgreSQL is the V0 source of truth. The immutable signal journal and semantic
outbox retain partition keys and event timestamps so CDC can later feed a
Kafka/Flink projection path without changing producer contracts.

Database tests are destructive and require a database whose name contains
`trust_risk_test`:

```bash
NVBES_TRUST_RISK_DATABASE_URL=postgres://localhost/nvbes_trust_risk_test \
  pnpm nx run trust-risk-service:test:database
```

The `erase-subject` command removes the requested subject links and active
features, then records an audited rebuild request. It requires an explicit
subject kind, namespace, opaque ID, actor and reason.
