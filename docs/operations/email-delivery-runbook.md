# Global email delivery runbook

## Runtime topology

Run `nvbes-email-worker` as an always-on container. HTTP/1.1 and gRPC/h2c share
the single runtime port `3040`, while the process owns three concurrent roles:

- authenticated gRPC ingestion on `3040`;
- public HTTP webhook and probes on `3040`;
- PostgreSQL-backed dispatch loop.

Do not deploy it as a scale-to-zero request-only job: the dispatcher must make
progress without an inbound request. Product services use gRPC only. HTTP is
reserved for Scaleway Topics and Events and health probes.

## Health ownership

- The container runtime calls `GET /health/live`. This is shallow and must not
  depend on PostgreSQL or Scaleway TEM.
- The deployment workflow calls `GET /health/ready`. It requires a reachable,
  migrated database and a current dispatcher heartbeat.
- Internal gRPC diagnostics may call `grpc.health.v1.Health` on `3040`.
- Scaleway probes never call the gRPC health service.

The TEM provider is deliberately absent from readiness. A TEM outage must
accumulate durable commands instead of evicting every ingress instance.

## Required production configuration

Configure the runtime with:

- `NVBES_EMAIL_DATABASE_URL`;
- `NVBES_EMAIL_PRODUCER_TOKENS`, with one distinct `producer=token` entry per
  production producer;
- `NVBES_EMAIL_DATA_ENCRYPTION_KEY` and `NVBES_EMAIL_RECIPIENT_HMAC_KEY`;
- `NVBES_EMAIL_PAYLOAD_RETENTION_DAYS` and `NVBES_EMAIL_LEDGER_RETENTION_DAYS`;
- `NVBES_EMAIL_FROM_EMAIL`, sender name, and optional reply-to;
- `NVBES_SCALEWAY_EMAIL_PROJECT_ID`, secret key, and `fr-par` region;
- `NVBES_EMAIL_SNS_TOPIC_ARN`;
- `NVBES_EMAIL_SNS_CA_BUNDLE_PATH` containing the pinned Scaleway SNS trust
  chain.
- `SENTRY_DSN` for server-side error reporting and `SENTRY_RELEASE` for release
  attribution; keep `SENTRY_TRACES_SAMPLE_RATE` between `0` and `1` (the
  production default remains `0` until performance sampling is explicitly
  budgeted).

Mount secrets read-only. Product services receive only the gRPC endpoint and
internal authentication token; they never receive provider or encryption
credentials.

Each product process receives its own value through
`NVBES_EMAIL_GRPC_AUTH_TOKEN`. The email runtime receives the complete mapping
through `NVBES_EMAIL_PRODUCER_TOKENS`. Reusing one token for multiple producers
is rejected outside development and test.

## Metrics and alerts

Scrape `GET /metrics` only with the dedicated internal bearer token. The path
shares the Serverless listener but returns no metrics without that token; only
the observability collector receives it. The runtime emits queue depth and age,
command outcomes, provider latency, webhook outcomes, signature failures,
expirations, suppressions, and retention activity.

Alerts cover stale queue age, elevated provider failures, and spam complaint
rate. During staging rehearsals, exercise each alert and attach the evidence to
the release packet.

Validate Sentry after each environment is wired by running
`nvbes-email-worker error-reporting-smoke`. The command returns a JSON event ID,
monitor check-in ID, configuration state, and flush result without connecting
to PostgreSQL or starting the delivery runtime. The running dispatcher also
checks in to the `email-worker-dispatcher-heartbeat` Sentry monitor every five
minutes.

## Deployment order

The production sender domain is the dedicated transactional subdomain
`notify.nvbes.eu`. Configure `NVBES_EMAIL_FROM_EMAIL` as
`no-reply@notify.nvbes.eu` and `NVBES_EMAIL_MESSAGE_ID_DOMAIN` as
`notify.nvbes.eu`. Leave `NVBES_EMAIL_REPLY_TO` unset until inbound delivery for
`support@nvbes.eu` exists.

Cloudflare remains authoritative for DNS, while
`infrastructure/stacks/email/production/email-delivery.tf` is the sole writer.
Terraform registers `notify.nvbes.eu` in Scaleway TEM, publishes the exact SPF,
DKIM, DMARC, and blackhole MX values returned by TEM, requests validation,
creates the SNS topic and HTTPS subscription, then binds the TEM webhook. Do
not copy example record values into production and do not mutate these
resources through either provider console, `scw`, or Wrangler. Import any
pre-existing resource into the production state before applying the stack.
The apex `nvbes.eu` remains separate so Cloudflare Email Routing can later
receive human mail without replacing the transactional subdomain MX records.

1. Apply the email database migration.
2. Start the container and wait for `/health/live` then `/health/ready` so its
   public HTTPS webhook can accept the SNS subscription confirmation.
3. Initialize the required remote backend and apply the production Terraform
   stack with `accept_scaleway_tem_terms=true`.
4. Confirm the TEM domain validation and SNS subscription outputs/state.
5. Send one short-lived test command and confirm provider acceptance plus the
   webhook transition.
6. Enable product relays.

Never run a production apply with a local state. The production README defines
the backend bootstrap and credential contract. Terraform provider credentials
must come from the deployment secret manager, not committed tfvars.

The repository publishes and signs the container image, but the manual deploy
workflow intentionally fails until an always-on Scaleway target and Load
Balancer are provisioned. Never replace that failure with a placeholder
success.

## Deadline incidents

A credential email must use the authoritative credential expiry as
`deliver_before`. The dispatcher checks database time before claiming and
again immediately before the provider call. When the deadline prevents the
next attempt, the message becomes terminal `expired`; it is not retried.

Provider-accepted messages never transition to `expired` later. Their embedded
credential may expire, but the delivery itself already occurred.

## Provider incidents

Keep the runtime ready during a TEM outage. Commands remain durable and retry
within their category deadline. Investigate `email_delivery_duration_seconds`
and `email_messages_total` by outcome. Do not bypass the worker or send directly
through SMTP or the provider API.

## Deliverability incidents

Pause non-critical producers when hard-bounce or complaint rates cross their
alert thresholds. Verify the sender domain, SPF, DKIM, DMARC, TEM reputation,
and the signed webhook subscription before resuming volume. Scaleway TEM does
not provide a transactional unsubscribe event; optional-mail preference flows
must create an audited optional suppression inside nvbes rather than inventing
a provider event type.

## Retention and suppression release

Terminal recipient and template ciphertext is purged after
`NVBES_EMAIL_PAYLOAD_RETENTION_DAYS`. The non-secret lifecycle ledger is removed
after `NVBES_EMAIL_LEDGER_RETENTION_DAYS`.

The normal operator path is the Backoffice communications center. It calls the
authenticated operations service and records both the worker-owned action audit
and the correlated Backoffice audit event.

For break-glass recovery when Backoffice is unavailable, use a historical email
message UUID so no address is placed in shell history:

```bash
nvbes-email-worker release-suppression \
  00000000-0000-0000-0000-000000000000 \
  operator-principal-id \
  "verified address change"
```

The operation records the actor, reason, and release timestamp. It refuses to
succeed when no active suppression matches the message recipient.

## Legacy schema cutover

Identity migration `0060_drop_legacy_email_delivery.sql` and Billing migration
`0013_drop_legacy_email_delivery.sql` remove their former `email_messages`,
`email_events`, and `suppressed_emails` tables. Apply them only after the email
worker database and authenticated operations/privacy routes are healthy. Follow
the standard encrypted database-backup policy before deployment because these
migrations intentionally delete historical clear-text recipient data.
