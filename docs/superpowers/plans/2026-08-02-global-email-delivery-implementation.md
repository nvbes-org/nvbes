# Global Email Delivery Implementation Plan

Implement the approved global email delivery design in dependency order, preserving existing product behavior until each producer is cut over.

## Phase 1 — Contract and shared package

1. Add `contracts/protobuf/nvbes/email/v1/email.proto` with:
   - `EmailDeliveryService.SubmitEmail`;
   - versioned typed template variants;
   - recipient, category, idempotency key, producer, request context, and `deliver_before`;
   - receipt fields for message ID, acceptance time, effective deadline, and duplicate state.
2. Extend `libs/rust/email` with protobuf generation and exports.
3. Add a tonic `EmailClient` that:
   - loads an HTTPS endpoint and bearer credential outside development;
   - attaches authorization metadata and a bounded call deadline;
   - exposes stable Rust command/receipt/error types;
   - maps gRPC statuses without leaking provider details.
4. Preserve the provider-neutral `EmailSender` trait temporarily for the Scaleway adapter and compatibility tests.
5. Add contract, conversion, validation, TLS-policy, and error-mapping tests.

Verification:

```bash
rtk cargo test -p nvbes-email --locked
rtk cargo check -p nvbes-email --all-targets --locked
```

## Phase 2 — Rust email runtime foundation

1. Replace the TypeScript `apps/email-worker` package with a Rust application:
   - `Cargo.toml`, `build.rs`, `project.json`, and Dockerfile;
   - flat, dot-qualified Rust modules under `src/`;
   - one process hosting tonic gRPC, Axum HTTP, and a dispatch loop.
2. Register `apps/email-worker` in the Cargo workspace.
3. Add an email-owned migration creating:
   - `email_messages`;
   - `email_delivery_attempts`;
   - `email_provider_events`;
   - `email_suppressions`;
   - enums, constraints, indexes, and idempotency uniqueness.
4. Implement strict runtime configuration for:
   - email database URL;
   - gRPC/HTTP bind addresses;
   - internal bearer token;
   - Scaleway project, secret, region, sender, and Topics and Events trust settings;
   - development mock/test-capture mode.
5. Implement `/health/live` and `/health/ready` with the approved dependency semantics.

Verification:

```bash
rtk pnpm nx show project email-worker --json
rtk cargo test -p nvbes-email-worker --locked
rtk cargo check -p nvbes-email-worker --all-targets --locked
```

## Phase 3 — Command ingestion and rendering

1. Implement gRPC bearer authentication with constant-time comparison and producer allowlisting.
2. Validate every command before persistence:
   - non-empty producer/idempotency/recipient;
   - known category/template pairing;
   - valid email syntax;
   - `deliver_before > database_now`;
   - credential template deadline not greater than its policy maximum.
3. Persist idempotently using `(producer, idempotency_key)` plus an immutable command fingerprint.
4. Return the original receipt for identical duplicates and `AlreadyExists` for conflicting reuse.
5. Move all canonical templates into the shared email boundary with:
   - versioned typed input;
   - escaped HTML;
   - hand-authored text;
   - preview text;
   - centrally controlled sender and reply-to.
6. Add tests for every template, escaping, deadline validation, and concurrent duplicate submission.

Verification:

```bash
rtk cargo test -p nvbes-email -p nvbes-email-worker --locked
```

## Phase 4 — Dispatch, retries, and delivery deadlines

1. Complete the Scaleway TEM REST adapter and require HTTPS/TLS outside tests.
2. Implement short transactional claims with `FOR UPDATE SKIP LOCKED` and committed leases.
3. Before claim and provider call, compare PostgreSQL time with `deliver_before`.
4. Transition an unaccepted stale message to terminal `expired` without a provider call.
5. Implement category retry policies with bounded attempts, exponential backoff, jitter, and `next_attempt_at < deliver_before`.
6. Reuse stable `Message-ID` and job metadata across attempts.
7. Record transient, permanent, and ambiguous outcomes per attempt.
8. Preserve provider-accepted state after the embedded credential expires.
9. Add concurrency, crash-window, retry, deadline, and provider-adapter tests.

Verification:

```bash
rtk cargo test -p nvbes-email-worker --locked
```

## Phase 5 — Scaleway webhook procedure

1. Implement the Topics and Events HTTP endpoint with strict method, content type, and body limits.
2. Support subscription confirmation with an allowlisted HTTPS URL.
3. Validate signing certificate URLs, the configured `fr-par` CA trust chain, certificate rotation, and SNS canonical signatures.
4. Deduplicate SNS and TEM event identifiers.
5. Normalize provider events and apply delivery transitions transactionally.
6. Implement global suppression rules for hard bounce, complaint, unsubscribe, soft-bounce threshold, and drop.
7. Store only bounded redacted event details.
8. Add fixture-driven signature, rotation, replay, ordering, and suppression tests.

Verification:

```bash
rtk cargo test -p nvbes-email-worker --locked
```

## Phase 6 — Producer cutover

1. Identity:
   - add `EmailClient` to exact application dependencies;
   - submit verification, password-reset, password-change-code, and account-security templates;
   - set code/token `deliver_before` to the authoritative credential expiry;
   - remove Identity Redis email send/webhook queues and product email delivery tables after migration.
2. Billing:
   - submit receipt and payment-failure templates from the billing event worker;
   - remove Billing SMTP sender, delivery implementation, and email queue;
   - preserve provider-event idempotency keys.
3. Enterprise reminders:
   - submit access-review reminders through the same gRPC client;
   - use an explicit reminder deadline policy.
4. Remove the orphan invitation template because no active producer owns an invitation email flow.
5. Ensure all product outbox relays invoke the same gRPC contract and no product owns provider credentials.

Verification:

```bash
rtk pnpm nx run-many -t check,test --projects=identity-service,identity-worker,billing-worker,enterprise-service,email-worker
```

## Phase 7 — Infrastructure and legacy removal

1. Replace Cloudflare worker deployment/CI with the Rust container workflow.
2. Add the email runtime container contract and release matrix entry.
3. Configure Scaleway private gRPC ingress and HTTP routes for webhook and health probes.
4. Configure `/health/ready` as the Scaleway Load Balancer check and `/health/live` as the container check.
5. Provision Topics and Events, TEM webhook subscription, sender-domain release gates, secrets, and email database/KMS resources exclusively through Terraform.
6. Remove:
   - TypeScript worker sources/configuration;
   - Cloudflare email pipeline Terraform;
   - duplicate TypeScript templates;
   - unused SMTP production paths and unreferenced email code.
7. Update runbooks, dashboards, secret inventories, and architecture docs.

Verification:

```bash
rtk pnpm check:structure
rtk pnpm check:product-boundaries
rtk cargo check --workspace --locked
rtk pnpm check
```

## Completion gates

- All code files remain below 500 lines and touched files are kept below 300 lines where practical.
- No new warning suppression or `any` type is introduced.
- No secret or recipient PII is logged.
- Every code/token email has an authoritative deadline test.
- No product service can dispatch directly to Scaleway or SMTP.
- No provider call begins at or after `deliver_before`.
- All skipped checks or environment-dependent validation are documented explicitly.

## Post-implementation cutover status

The global runtime, typed producers, delivery ledger, provider adapter,
authenticated webhook, suppression policy, retention, metrics, alerts, and
container release path are implemented in the repository.

The repository-side consumer cutover is complete:

- Backoffice communications, compliance, and operations views use the
  authenticated email operations service;
- operator replay and suppression actions are executed and audited by the
  email worker;
- the coordinated Account privacy export obtains retained email activity from
  the authenticated privacy RPC;
- Identity migration `0060` and Billing migration `0013` remove the historical
  recipient-level delivery tables and their enum types.

The sender-domain infrastructure is now declared in the production Terraform
stack. Terraform is the sole writer for the TEM domain, exact Cloudflare DNS
records, domain validation, SNS topic/subscription, and TEM webhook; manual
provider-console, `scw`, and Wrangler mutations are prohibited.

The final production cutover remains gated on applying and completing the
remaining runtime infrastructure workstream:

1. provision the always-on Scaleway target, private gRPC route, public signed
   webhook route, email database, and KMS resources;
2. initialize the encrypted remote state, import any pre-existing email
   resources, and apply the production stack after reviewing the TEM terms and
   deploying the webhook endpoint;
3. verify `notify.nvbes.eu`, the signed subscription handshake, and a
   short-lived end-to-end delivery. The apex `nvbes.eu` remains reserved for
   inbound human mail through Cloudflare Email Routing.

Before applying the two destructive legacy-schema migrations in an existing
environment, retain the normal encrypted database backup required by the
deployment policy. No product delivery code writes lifecycle data outside the
email database after this cutover.
