# Scaleway TEM Email Webhooks — Provider Email ID Tracking

## Goal
Add Scaleway TEM email delivery webhooks (delivered, dropped, mailbox_not_found, spam, deferred, etc.) with suppression list and provider email_id tracking to correlate webhook events with sent emails.

## Constraints & Preferences
- Follow existing billing webhook pattern (Stripe) for job enqueueing and worker processing
- Webhook route must bypass origin_guard middleware (server-to-server, no browser Origin)
- Use `hmac` + `sha2` for Scaleway HMAC-SHA256 signature verification
- Use existing `worker_jobs` table with `ON CONFLICT DO NOTHING` for idempotency
- New tables: `email_events`, `suppressed_emails` in a single migration
- Suppression check in `enqueue_email_job_tx` before inserting send job
- The `email_events` table also records the sent event (via `record_email_sent_event_tx`) so `provider_email_id` is the correlation key between send and webhook events
- `ScalewayEmailEvent` struct must include `email_id` field from Scaleway webhook payloads

## Progress
### Done
- Created migration `20260512210000_email_webhooks_v1.sql`: `email_event_type` enum, `email_events`, `suppressed_emails` tables
- Created migration `20260512210001_email_events_provider_email_id.sql`: adds `provider_email_id` column to `email_events` + index
- Added `scw_tem_webhook_secret` to `AppConfig` in `nvbes-core` (`config.rs`): field, `from_env()`, `resolve_from_secret_manager()`
- Added `hmac.workspace = true` to `apps/identity-api/Cargo.toml`
- Created `identity.domains.email.db.rs`: `record_email_event_tx`, `record_email_sent_event_tx`, `mark_email_event_processed_tx`, `suppress_email_tx`, `is_email_suppressed`, `build_event_details`
- Created `identity.domains.email.jobs.rs`: `JOB_EMAIL_WEBHOOK_PROCESS`, `enqueue_email_event_job_tx`, added `pool` param to `enqueue_email_job_tx` with suppression check
- Created `identity.domains.email.webhooks.handlers.rs`: `webhook_router`, `handle_scaleway_email_webhook` (signature verify, event record, job enqueue), `verify_scaleway_signature` (HMAC-SHA256), `is_actionable_event`; struct includes `email_id` field
- Created `identity.domains.email.webhooks.rs`: re-exports `handlers`
- Created `identity.domains.email.routes.rs`: empty `router()` for future email endpoints
- Updated `identity.domains.email.mod.rs`: added `db`, `routes`, `webhooks` modules
- Updated `identity.worker.rs`: `JOB_EMAIL_WEBHOOK_PROCESS` dispatch arm, `process_email_event` (suppress on bounce/spam/blacklist/unsubscribed/dropped, mark processed); JOB_EMAIL_SEND arm now captures `email_id` from `send_message` and records sent event in `email_events`
- Updated `identity.http.mod.rs`: merged `crate::email::webhooks::webhook_router(state)` after `origin_guard` layer (bypasses origin check)
- Updated `identity.domains.mod.rs`: added `email::routes::router(state)` merge
- Updated `identity.domains.auth.password.rs`: added `&state.db` arg to `enqueue_email_job_tx`
- Updated `identity.domains.auth.db.rs`: added `&state.db` arg to `enqueue_email_job_tx`
- Updated `identity.domains.members.invites.rs`: added `&state.db` arg to `enqueue_email_job_tx`
- Changed `EmailSender::send_message` trait signature to return `Result<String, EmailError>` (returns provider email_id)
- Updated `ScalewayEmailClient::send_message` to parse `email_id` from POST /emails response body
- Updated `MockEmailSender::send_message` to return dummy `mock-{to_email}` string
- Added `provider_email_id` column to `record_email_event_tx` INSERT (from `ScalewayEmailEvent.email_id`)
- `nvbes-identity-api` compiles clean (0 errors 0 warnings from email/webhook code; pre-existing utoipa `ToSchema` for `WorkspaceRole` are unrelated)

### In Progress
- (none)

### Blocked
- Pre-existing `drive-api` compilation errors (missing file `drive.domains.billing.helpers.rs`, utoipa `ToSchema` for `WorkspaceRole`) are unrelated and do not block identity-api

## Key Decisions
- Mount webhook route (`/webhooks/email/scaleway`) in `identity.http.mod.rs` after `.layer(origin_guard)` so server-to-server calls bypass origin validation; pattern: add `.merge()` after layer in Axum
- Use `email_events` table for full audit trail + `suppressed_emails` table for blocklist; the latter keeps "hard_bounce" reason immutable once set (lowest-priority override)
- Added `pool` parameter to `enqueue_email_job_tx` instead of using transaction connection, because `is_email_suppressed` queries a separate table (no transactional consistency needed) and `PgPool` is more explicit at call sites
- Suppress on: `email_mailbox_not_found` → `hard_bounce`, `email_spam` → `spam_complaint`, `email_bounced` → `bounced`, `email_blacklisted` → `bounced`, `email_unsubscribed` → `unsubscribed`, `email_dropped` → `dropped`. All others (delivered, deferred, opened, clicked, sent) are informational only.
- `send_message` now returns the `email_id` from Scaleway's response; the worker immediately records a `email_sent` event in `email_events` with that ID. Webhook handlers also populate `provider_email_id`, making `email_events.provider_email_id` the correlation key between send and post-send events.
- `record_email_sent_event_tx` uses a synthetic `provider_event_id` (`sent:{email_id}`) to avoid conflict with webhook event IDs.
- Clone `to_email` and `subject` from payload before building `EmailMessage` to avoid borrow-after-move (Rust ownership).

## Next Steps
1. Test end-to-end: send an email (run worker), then fire a mock webhook with matching `email_id`, verify the `email_events` table contains both the `email_sent` row and the webhook event row with the same `provider_email_id`
2. Validate that `email_delivered` and non-actionable webhook events are recorded but do not trigger suppression or additional processing
3. Verify the HMAC signature verification with a real Scaleway TEM webhook secret (integration environment)
4. Consider adding a query to correlate `email_events` by `provider_email_id` for debugging/monitoring

## Critical Context
- `send_message()` now returns `String` (the provider email_id), not `()`. Every caller must handle the ID.
- `AppConfig` is in `libs/rust/core/src/config.rs`; `SCW_TEM_WEBHOOK_SECRET` env var added.
- `email` module is declared at `crate::email` (root of identity-api lib), not `crate::domains::email`.
- `identity.http.mod.rs::router()` returns `Router<crate::app::AppState>`; final `.with_state()` call is in `identity.app.rs::build_router()`.
- The origin_guard allows requests with `Authorization` header or matching `Origin`/`Referer`. Webhooks have neither, so they must bypass it.
- Scaleway TEM signature format: `sha256=<hex>` in `X-Scaleway-Signature` header, HMAC-SHA256 over raw body.
- Worker has `run_loop_until_shutdown` → `run_once` → `claim_next_job` → `execute_job` → match on `job.job_type`.
- `nvbes-identity-api` compiles clean for email/webhook code (0 errors, 0 warnings). Pre-existing drive-api and utoipa errors are unrelated.
- `AppError` does not implement `std::error::Error`; format strings must use `{:?}` and `map_err` with `anyhow!` when converting to `anyhow::Error` in the worker.

## Relevant Files
- `apps/identity-api/migrations/20260512210000_email_webhooks_v1.sql`: initial migration for email_events + suppressed_emails
- `apps/identity-api/migrations/20260512210001_email_events_provider_email_id.sql`: adds `provider_email_id` column + index
- `apps/identity-api/src/identity.domains.email.db.rs`: email_events/suppressed_emails DB operations (incl. `record_email_sent_event_tx`)
- `apps/identity-api/src/identity.domains.email.jobs.rs`: job type constants, enqueue functions, suppression check
- `apps/identity-api/src/identity.domains.email.webhooks.handlers.rs`: webhook handler + signature verification; parses `email_id`
- `apps/identity-api/src/identity.domains.email.webhooks.handlers.rs`: `ScalewayEmailEvent` struct with `email_id` field
- `apps/identity-api/src/identity.worker.rs`: email send + webhook event processing; returns `provider_email_id` on send
- `apps/identity-api/src/identity.http.mod.rs`: webhook route merged after origin_guard
- `apps/identity-api/src/identity.domains.mod.rs`: email routes merged into domain router
- `libs/rust/core/src/config.rs`: `scw_tem_webhook_secret` config
- `libs/rust/email/src/trait_def.rs`: `EmailSender::send_message` now returns `Result<String, EmailError>`
- `libs/rust/email/src/scaleway.rs`: parses `email_id` from POST /emails response
- `libs/rust/email/src/mock.rs`: returns dummy `mock-{to_email}` email_id
- `apps/identity-api/Cargo.toml`: `hmac.workspace = true` added
