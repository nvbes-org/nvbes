# Trust/Risk Service V0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an independent Rust Trust/Risk runtime with versioned gRPC contracts, PostgreSQL-backed signal ingestion, replayable feature projection, deterministic scoring, labels, review cases, rule operations, retention, audit and Billing checkout acceptance fixtures.

**Architecture:** A provider-neutral `nvbes-trust-risk` crate owns public contracts, validation and client behavior. One `nvbes-trust-risk-service` binary hosts authenticated gRPC ingress, a PostgreSQL projection loop, operations APIs and HTTP health/metrics; raw signals and evaluations are immutable while all feature state is reconstructible. PostgreSQL remains the V0 source of truth, and event envelopes preserve partition and event-time semantics for later CDC into Kafka/Flink.

**Tech Stack:** Rust 2024, tonic/prost, Axum, Tokio, SQLx/PostgreSQL, Serde, SHA-256, metrics, Nx run-command projects, Cargo workspace.

---

## File map

### Public contracts and client

- `contracts/protobuf/nvbes/trust_risk/v1/trust_risk.proto`: all V1 producer and operations wire contracts.
- `libs/rust/trust-risk/Cargo.toml`: provider-neutral client crate dependencies.
- `libs/rust/trust-risk/build.rs`: vendored protoc generation.
- `libs/rust/trust-risk/project.json`: Nx project metadata.
- `libs/rust/trust-risk/src/lib.rs`: explicit exports only.
- `libs/rust/trust-risk/src/proto.rs`: generated module tree.
- `libs/rust/trust-risk/src/trust_risk.types.rs`: validated domain enums and bounded primitives.
- `libs/rust/trust-risk/src/trust_risk.signal.rs`: signal/subject/attribute conversion and validation.
- `libs/rust/trust-risk/src/trust_risk.assessment.rs`: assessment/evaluation conversion and validation.
- `libs/rust/trust-risk/src/trust_risk.label.rs`: label conversion and validation.
- `libs/rust/trust-risk/src/trust_risk.rules.rs`: typed rule AST, static validation and pure evaluator.
- `libs/rust/trust-risk/src/trust_risk.client.rs`: authenticated gRPC client and typed error mapping.
- sibling `*.tests.rs` files: focused unit, property and integration tests.

### Runtime

- `apps/trust-risk-service/Cargo.toml`: runtime dependencies and database-test feature.
- `apps/trust-risk-service/project.json`: Nx check/test/database-test targets.
- `apps/trust-risk-service/README.md`: local runtime and contract documentation.
- `apps/trust-risk-service/migrations/0001_trust_risk_foundation.sql`: source journal, projections, rules, evaluations, labels, review, audit, failure and retention schema.
- `apps/trust-risk-service/src/main.rs`: composition root only.
- `apps/trust-risk-service/src/trust_risk.app.rs`: shared runtime state and task handles.
- `apps/trust-risk-service/src/trust_risk.config.rs`: strict environment and producer-policy parsing.
- `apps/trust-risk-service/src/trust_risk.auth.rs`: constant-time workload and operator authentication plus ACL checks.
- `apps/trust-risk-service/src/trust_risk.database.rs`: pool, migrations and database clock.
- `apps/trust-risk-service/src/trust_risk.ingress.db.rs`: idempotent signal persistence.
- `apps/trust-risk-service/src/trust_risk.ingress.grpc.rs`: bounded `SubmitSignals` endpoint.
- `apps/trust-risk-service/src/trust_risk.projection.db.rs`: claim, recompute, success, retry and quarantine persistence.
- `apps/trust-risk-service/src/trust_risk.projection.rs`: projection loop and deterministic feature derivation.
- `apps/trust-risk-service/src/trust_risk.assessment.db.rs`: atomic evidence and evaluation ledger persistence.
- `apps/trust-risk-service/src/trust_risk.assessment.grpc.rs`: `AssessRisk` endpoint.
- `apps/trust-risk-service/src/trust_risk.labels.db.rs`: append-only assertions and canonical resolution.
- `apps/trust-risk-service/src/trust_risk.labels.grpc.rs`: `SubmitLabels` endpoint.
- `apps/trust-risk-service/src/trust_risk.review.db.rs`: case state machine and append-only events.
- `apps/trust-risk-service/src/trust_risk.operations.grpc.rs`: evaluation, review and rule operations.
- `apps/trust-risk-service/src/trust_risk.rules.db.rs`: stage, activate and rollback with dual control.
- `apps/trust-risk-service/src/trust_risk.retention.rs`: policy enforcement, erasure and legal-hold behavior.
- `apps/trust-risk-service/src/trust_risk.audit.rs`: append-only risk audit writes.
- `apps/trust-risk-service/src/trust_risk.health.rs`: liveness and dependency-aware readiness.
- `apps/trust-risk-service/src/trust_risk.metrics.rs`: bounded Prometheus metrics.
- `apps/trust-risk-service/src/trust_risk.test_support.rs`: database fixtures and authenticated requests.
- sibling `*.tests.rs` files: unit/database/E2E tests under 300 lines each.

### Workspace and verification

- `Cargo.toml`, `Cargo.lock`: add both workspace members and resolved dependencies.
- `tools/rust-workspace/project.json`: include the new app and library in Nx dependencies/inputs.
- `scripts/test-trust-risk-service-database.sh`: isolated database-test runner.
- `scripts/validate-trust-risk-test-database.mjs`: reject unsafe database targets.
- `.env.example`: document development-only Trust/Risk variables without secrets.

## Task 1: Scaffold contracts and workspace projects

**Files:**
- Create every contract/client/runtime manifest and project file listed above.
- Modify: `Cargo.toml`
- Modify: `tools/rust-workspace/project.json`

- [x] **Step 1: Verify no Rust Nx generator exists**

Run:

```bash
rtk pnpm nx list nx
rtk find tools -maxdepth 4 -type f | rtk rg 'generator|generators.json'
```

Expected: only Nx setup generators and no local Rust application/library generator. Manual scaffolding is therefore required.

- [x] **Step 2: Add the protobuf contract**

Create `trust_risk.proto` with four services and typed messages:

```proto
service TrustRiskSignalService {
  rpc SubmitSignals(SubmitSignalsRequest) returns (SubmitSignalsResponse);
}
service TrustRiskAssessmentService {
  rpc AssessRisk(AssessRiskRequest) returns (RiskEvaluation);
}
service TrustRiskLabelService {
  rpc SubmitLabels(SubmitLabelsRequest) returns (SubmitLabelsResponse);
}
service TrustRiskOperationsService {
  rpc GetEvaluation(GetEvaluationRequest) returns (RiskEvaluationDetail);
  rpc ListReviewCases(ListReviewCasesRequest) returns (ListReviewCasesResponse);
  rpc TransitionReviewCase(TransitionReviewCaseRequest) returns (ReviewCase);
  rpc StageRuleSet(StageRuleSetRequest) returns (RuleSetReceipt);
  rpc ActivateRuleSet(ActivateRuleSetRequest) returns (RuleSetReceipt);
  rpc RollbackRuleSet(RollbackRuleSetRequest) returns (RuleSetReceipt);
}
```

Define `RiskSignal`, `SubjectReference`, `AttributeValue`, `AssessRiskRequest`, `RiskEvaluation`, `RiskLabel`, operator context, review messages and bounded canonical JSON rule-set messages. Reserve enum zero values as unspecified.

- [x] **Step 3: Add manual Nx/Cargo scaffolding**

Add workspace members:

```toml
"apps/trust-risk-service",
"libs/rust/trust-risk",
```

Create `project.json` targets using `nx:run-commands`:

```json
{
  "check": { "options": { "command": "cargo check --locked --package nvbes-trust-risk-service --all-targets" } },
  "test": { "options": { "command": "cargo test --locked --package nvbes-trust-risk --package nvbes-trust-risk-service" } },
  "test:database": { "options": { "command": "bash scripts/test-trust-risk-service-database.sh" } }
}
```

Use tags `type:app,domain:trust-risk,layer:app` for the service and `type:lib,domain:trust-risk,layer:core` for the crate.

- [x] **Step 4: Add protobuf generation and empty module roots**

`build.rs` must compile `common.proto` and `trust_risk.proto` with vendored protoc. `proto.rs` must expose `nvbes::platform::v1` and `nvbes::trust_risk::v1`. `lib.rs` and `main.rs` declare only explicit flat modules.

- [x] **Step 5: Run the first compile**

Run:

```bash
rtk pnpm nx run trust-risk-service:check
```

Expected: PASS with generated protobuf modules available.

- [x] **Step 6: Commit the scaffold**

```bash
rtk git add Cargo.toml Cargo.lock contracts/protobuf/nvbes/trust_risk apps/trust-risk-service libs/rust/trust-risk tools/rust-workspace/project.json
rtk git commit -S -m "feat(trust-risk): scaffold service contracts"
```

## Task 2: Implement validated domain types and rule engine

**Files:**
- Create/modify the public crate domain and test files.

- [x] **Step 1: Write failing validation tests**

Cover an opaque subject, forbidden email-like subject, valid namespaced signal, unknown attribute, score bounds, unspecified enums, rule nesting depth and deterministic ordering:

```rust
#[test]
fn rejects_email_like_subject_identifiers() {
    let subject = subject("ada@example.com");
    assert_eq!(SubjectReference::try_from(subject).unwrap_err(), SignalError::ForbiddenSubject);
}

#[test]
fn evaluator_is_deterministic_and_bounded() {
    let first = evaluate(&rules(), &features());
    let second = evaluate(&rules(), &features());
    assert_eq!(first, second);
    assert!(first.score <= 100);
}
```

- [x] **Step 2: Verify the tests fail**

Run `rtk cargo test -p nvbes-trust-risk --lib`.

Expected: FAIL because the domain modules and conversions are not implemented.

- [x] **Step 3: Implement bounded primitives and conversions**

Implement `SignalKind`, `OpaqueSubjectId`, `AssessmentKey`, `ReasonCode`, `RiskScore`, `RiskBand`, `Recommendation`, `LabelKind` and request conversion. Each constructor trims input, enforces documented lengths/character sets and returns a typed `thiserror` error.

Use an explicit per-family attribute schema:

```rust
pub fn allowed_attribute(signal_kind: &str, key: &str) -> bool {
    matches!(
        (signal_kind.split('.').next(), key),
        (Some("network"), "risk_score" | "network_kind")
            | (Some("automation"), "confidence" | "detected")
            | (Some("payment"), "outcome" | "provider_risk_score")
            | (Some("reputation"), "confidence" | "trusted")
    )
}
```

- [x] **Step 4: Implement the pure typed rule engine**

Deserialize canonical JSON into `#[serde(deny_unknown_fields)]` types. Support bounded `all`, `any`, `not` and comparison predicates, score deltas, stable reasons and minimum recommendations. Reject depth over eight, more than 256 rules, duplicate rule/reason codes, non-finite thresholds and invalid recommendation thresholds.

Evaluation returns an ordered, deduplicated reason list and clamps the final score to `0..=100`.

- [x] **Step 5: Run domain tests**

Run `rtk cargo test -p nvbes-trust-risk --lib`.

Expected: PASS.

- [x] **Step 6: Commit**

```bash
rtk git add libs/rust/trust-risk
rtk git commit -S -m "feat(trust-risk): add validated risk domain"
```

## Task 3: Implement the typed client

**Files:**
- Create `trust_risk.client.rs` and its unit/integration tests.

- [x] **Step 1: Write failing client tests**

Test HTTPS enforcement outside development, token length, positive timeouts, authorization metadata, timeout propagation and status mapping:

```rust
#[test]
fn production_requires_https() {
    let error = TrustRiskClientConfig::from_values(
        "production", "http://risk:3050".into(), TOKEN.into(), Duration::from_millis(100),
    ).unwrap_err();
    assert!(matches!(error, TrustRiskClientError::Configuration(_)));
}
```

- [x] **Step 2: Verify failure**

Run `rtk cargo test -p nvbes-trust-risk client`.

Expected: FAIL because the client is missing.

- [x] **Step 3: Implement client configuration and calls**

Use `NVBES_TRUST_RISK_GRPC_ENDPOINT`, `NVBES_TRUST_RISK_GRPC_AUTH_TOKEN`, TLS roots outside development, a default 100 ms assessment timeout and a 3 second durable-ingest timeout. Clone tonic clients per call, attach Bearer metadata and set request timeout.

Map invalid input, conflict, unauthorized, unavailable and protocol errors without exposing remote payloads.

- [x] **Step 4: Run tests and commit**

Run `rtk cargo test -p nvbes-trust-risk client` and commit:

```bash
rtk git add libs/rust/trust-risk
rtk git commit -S -m "feat(trust-risk): add authenticated client"
```

## Task 4: Add runtime configuration, auth and health

**Files:**
- Create runtime config/auth/app/database/health/metrics files and tests.

- [x] **Step 1: Write failing configuration and ACL tests**

Test development defaults, mandatory production database/token/policy values, minimum token length, duplicate policy rejection, exact bearer matching, signal-family ACL, assessment permission and operator permission.

- [x] **Step 2: Verify failure**

Run `rtk cargo test -p nvbes-trust-risk-service config auth health`.

- [x] **Step 3: Implement strict configuration**

Parse `NVBES_TRUST_RISK_PRODUCER_POLICIES` into explicit producer policies containing token, signal prefixes and permissions. Parse a distinct `NVBES_TRUST_RISK_OPERATOR_TOKENS` map. Refuse production startup without reviewed retention days for signals, evaluations, labels, reviews and audit.

Development defaults may use only fixed non-secret test values and localhost binds.

- [x] **Step 4: Implement state, auth and health**

State contains only `Arc<TrustRiskConfig>`, `PgPool`, metrics handle and projection heartbeat. Authentication compares exact Bearer tokens in constant time. `/health/live` is shallow; `/health/ready` checks migrations, active rules and projection heartbeat using bounded database calls.

- [x] **Step 5: Run tests and commit**

```bash
rtk cargo test -p nvbes-trust-risk-service config
rtk cargo test -p nvbes-trust-risk-service auth
rtk cargo test -p nvbes-trust-risk-service health
rtk git add apps/trust-risk-service
rtk git commit -S -m "feat(trust-risk): add secure runtime shell"
```

## Task 5: Create the PostgreSQL foundation and idempotent ingestion

**Files:**
- Create migration, ingestion DB/GRPC modules and database test tooling.

- [x] **Step 1: Write the migration and database contract tests**

The migration must create all tables from the design with checks, unique idempotency constraints, foreign keys and claim indexes. Tests assert that a new signal inserts once, an identical retry is duplicate, a changed retry conflicts and forbidden data never reaches SQL.

- [x] **Step 2: Add safe database-test validation**

The validation script accepts only a database name containing `trust_risk_test` and rejects production-like hosts unless the explicit repository destructive-test guard is present. The shell runner loads `.env` as data, exports the Trust/Risk test URL, validates it and runs the feature-gated tests serially.

- [x] **Step 3: Verify tests fail**

Run `rtk pnpm nx run trust-risk-service:test:database`.

Expected: FAIL because persistence is not implemented; if no local database is configured, record that environmental limitation and continue with compile/unit tests.

- [x] **Step 4: Implement idempotent persistence**

Fingerprint the canonical protobuf bytes with SHA-256. Insert signal, subjects and semantic outbox rows in one transaction. On unique conflict, compare fingerprints in constant time and return the original receipt only for identical content.

- [x] **Step 5: Implement bounded gRPC ingestion**

Authenticate against the request producer, authorize every signal kind/scope, validate domain conversion before SQL and enforce 256 KiB transport plus 192 KiB persistence budgets. Return per-signal stable receipts.

- [x] **Step 6: Run tests and commit**

```bash
rtk cargo test -p nvbes-trust-risk-service ingress
rtk pnpm nx run trust-risk-service:test:database
rtk git add apps/trust-risk-service scripts
rtk git commit -S -m "feat(trust-risk): persist canonical signals"
```

## Task 6: Implement replayable feature projection

**Files:**
- Create projection DB/logic modules and tests.

- [x] **Step 1: Write failing projection tests**

Cover concurrent claims, lease expiry, recompute idempotency, event-time 1h/24h windows, distinct principal/tenant reuse, allowed lateness, too-late metric, retry backoff and quarantine after the configured attempt limit.

- [x] **Step 2: Verify failure**

Run `rtk cargo test -p nvbes-trust-risk-service projection`.

- [x] **Step 3: Implement claim and recompute**

Use one atomic CTE with `FOR UPDATE SKIP LOCKED` to lease due events. Recompute each affected subject from retained immutable rows into a versioned feature map containing `events_1h`, `events_24h`, `high_risk_events_1h`, `linked_principals_24h`, `linked_tenants_24h`, `automation_confidence_max_1h`, `negative_labels` and `positive_labels`.

Write feature state and mark the source event projected in one transaction. Re-running must produce byte-equivalent canonical feature JSON.

- [x] **Step 4: Implement loop and failure policy**

Poll with shutdown watch, refresh a monotonic heartbeat after successful cycles, use bounded exponential retry with jitter and quarantine deterministic failures without deleting the source event.

- [x] **Step 5: Run tests and commit**

```bash
rtk cargo test -p nvbes-trust-risk-service projection
rtk git add apps/trust-risk-service
rtk git commit -S -m "feat(trust-risk): project replayable features"
```

## Task 7: Implement atomic assessments and evaluation ledger

**Files:**
- Create assessment DB/GRPC files and tests.

- [x] **Step 1: Write failing assessment tests**

Test evidence plus evaluation rollback, current feature loading, instantaneous evidence contribution, stable idempotent replay across rule activation, conflicting assessment reuse, score/reason persistence and unavailable error mapping.

- [x] **Step 2: Verify failure**

Run `rtk cargo test -p nvbes-trust-risk-service assessment`.

- [x] **Step 3: Seed and load the baseline shared rule set**

The migration stages and activates a reviewed V1 rule set with generic rules for high network risk, high automation confidence, subject reuse, high velocity, negative labels and bounded positive reputation. Thresholds map final score to `allow`, `challenge`, `review`, `deny` without Billing-specific fields.

- [x] **Step 4: Implement atomic evaluation**

Within one transaction, persist validated instantaneous signals, combine current subject feature maps with instantaneous evidence, evaluate the active immutable rule set and insert evaluation, exact feature snapshot and ordered reasons. Store request fingerprint and return the original row for identical retries.

- [x] **Step 5: Implement authenticated gRPC assessment**

Apply assessment ACL, size limit and deadline. Return typed statuses and never manufacture an allow result during an error.

- [x] **Step 6: Run tests and commit**

```bash
rtk cargo test -p nvbes-trust-risk-service assessment
rtk git add apps/trust-risk-service
rtk git commit -S -m "feat(trust-risk): add explainable assessments"
```

## Task 8: Implement labels and review cases

**Files:**
- Create label/review DB/GRPC and tests.

- [ ] **Step 1: Write failing tests**

Cover automatic source ACL, human operator ACL, immutable correction chains, canonical precedence, weak heuristics excluded from authoritative resolution, idempotent `review` case creation and valid/invalid state transitions.

- [ ] **Step 2: Verify failure**

Run `rtk cargo test -p nvbes-trust-risk-service labels review`.

- [ ] **Step 3: Implement append-only label resolution**

Insert assertions with source-scoped idempotency. Recompute canonical resolution in the same transaction using precedence `human > authoritative external > verified product`; retain heuristics without resolving the label. A correction must reference an existing assertion and cannot form a cycle.

- [ ] **Step 4: Implement review state machine**

Create one case per review evaluation. Permit only `open -> in_review -> resolved|inconclusive`; resolved requires an authoritative label. Append every transition with actor and reason.

- [ ] **Step 5: Run tests and commit**

```bash
rtk cargo test -p nvbes-trust-risk-service labels
rtk cargo test -p nvbes-trust-risk-service review
rtk git add apps/trust-risk-service
rtk git commit -S -m "feat(trust-risk): add labels and review workflow"
```

## Task 9: Implement rule operations, audit, retention and erasure

**Files:**
- Create operations/rules/audit/retention modules and tests.

- [ ] **Step 1: Write failing operations tests**

Test safe evaluation detail, operator auth, stage validation, activation by a different actor, rollback to a validated version, append-only audit, retention class enforcement, legal-hold exemption and subject erasure/rebuild scheduling.

- [ ] **Step 2: Verify failure**

Run `rtk cargo test -p nvbes-trust-risk-service operations retention`.

- [ ] **Step 3: Implement rule lifecycle and dual control**

Stage canonical typed JSON with checksum and creator. Activation requires a distinct authenticated actor and atomically retires the active version, activates the staged version and writes audit. Rollback follows the same dual-control rule and activates an already validated version.

- [ ] **Step 4: Implement operations reads and review transitions**

Return only safe feature names/values and reason codes. Do not return correlation keys, other-tenant identifiers, tokens or raw signal attributes.

- [ ] **Step 5: Implement retention and erasure**

Delete projected signals past their configured class deadline only when no legal hold applies. Purge feature rows after window plus replay margin. Retain evaluation/label/review/audit rows per class policy. Erasure removes subject links, deletes active feature projections and appends a rebuild request/audit event.

- [ ] **Step 6: Run tests and commit**

```bash
rtk cargo test -p nvbes-trust-risk-service operations
rtk cargo test -p nvbes-trust-risk-service retention
rtk git add apps/trust-risk-service
rtk git commit -S -m "feat(trust-risk): add governed operations"
```

## Task 10: Compose runtime, observability and Billing acceptance fixtures

**Files:**
- Modify composition root/app/metrics/README and create acceptance tests.

- [ ] **Step 1: Write failing end-to-end fixtures**

Use a test producer named `billing-checkout-fixture` with canonical signals only. Cover legitimate residential use, elevated network risk, coordinated device/network reuse, repeated attempts, chargeback feedback, human false-positive correction, idempotent retries and explicit simulated fail-open on service outage.

- [ ] **Step 2: Compose the runtime**

Run migrations only through the explicit `migrate` command. Normal startup uses lazy connection, installs safe tracing/metrics, starts the projection loop, mounts four gRPC services plus gRPC health and HTTP health/metrics on one Axum listener, and shuts down without abandoning accepted records.

- [ ] **Step 3: Add bounded metrics**

Record accepted/duplicate/rejected/conflicting signals, assessment duration/outcomes, processor lag/retry/quarantine, active versions, score bands/recommendations, review backlog and label coverage. Never use subject, request or evaluation IDs as metric labels.

- [ ] **Step 4: Run E2E and contract tests**

```bash
rtk cargo test -p nvbes-trust-risk-service acceptance
rtk pnpm nx run trust-risk-service:test
rtk pnpm nx run trust-risk-service:check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add apps/trust-risk-service libs/rust/trust-risk .env.example
rtk git commit -S -m "feat(trust-risk): complete independent runtime"
```

## Task 11: Final verification and documentation

**Files:**
- Modify README/spec only for factual implementation notes discovered during execution.

- [ ] **Step 1: Format and run targeted checks**

```bash
rtk cargo fmt --all --check
rtk pnpm nx run trust-risk-service:test
rtk pnpm nx run trust-risk-service:check
rtk pnpm check:structure
rtk pnpm check:nx-boundaries
rtk git diff --check
```

Expected: PASS.

- [ ] **Step 2: Run workspace verification**

```bash
rtk cargo check --workspace --locked
rtk pnpm check
```

Expected: PASS. If database tests were skipped solely because no isolated PostgreSQL target exists, document the exact missing environment variable and keep all compile/unit/E2E-without-database tests green.

- [ ] **Step 3: Self-review against the design completion criteria**

Confirm public contract versioning, reproducible evaluations, projection rebuild equivalence, absence of product DTOs, ACL/isolation/redaction, retention/erasure behavior, latency instrumentation and streaming migration invariants.

- [ ] **Step 4: Commit final verification documentation if changed**

```bash
rtk git add docs apps/trust-risk-service/README.md
rtk git commit -S -m "docs(trust-risk): document runtime verification"
```

Skip this commit when verification produces no documentation change.
