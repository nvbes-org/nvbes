# Global email delivery runbook

## Runtime topology

Deploy the same `nvbes-email-worker` image as two Scaleway Serverless Containers
with `min_scale=0`: a public `ingress` role and a private `dispatch` role. Each
uses one HTTP/2 port, and together they own three request-driven roles:

- authenticated gRPC command ingestion;
- signed TEM webhook ingestion and shallow HTTP probes;
- private SQS-triggered dispatch of one opaque email UUID per request.

There is no PostgreSQL polling loop. A successful command transaction publishes
only its UUID to Scaleway SQS. The SQS trigger wakes the container, which claims
that exact ledger row and calls TEM. Development uses an in-memory event channel
with the same handler; it waits for events and never polls the database.

## Health ownership

- The container runtime calls `GET /health/live`. This is shallow and must not
  depend on PostgreSQL or Scaleway TEM.
- Scaleway calls `GET /health/ready`. It is also shallow: probes must never wake
  PostgreSQL while the service is idle.
- Internal gRPC diagnostics may call `grpc.health.v1.Health` on the same port.
- Scaleway probes never call the gRPC health service.

The TEM provider is deliberately absent from readiness. A TEM outage must
accumulate durable commands instead of evicting every ingress instance.

## Required production configuration

Terraform creates a dedicated Scaleway Serverless SQL database for
`email-worker` and generates `NVBES_EMAIL_DATABASE_URL` from its data-only IAM
identity. Do not supply a shared RDB URL. Configure the remaining runtime with:

- `NVBES_EMAIL_PRODUCER_TOKENS`, with one distinct `producer=token` entry per
  production producer;
- `NVBES_EMAIL_DATA_ENCRYPTION_KEY` and `NVBES_EMAIL_RECIPIENT_HMAC_KEY`;
- `NVBES_EMAIL_PAYLOAD_RETENTION_DAYS` and `NVBES_EMAIL_LEDGER_RETENTION_DAYS`;
- `NVBES_EMAIL_FROM_EMAIL`, sender name, and optional reply-to;
- `NVBES_SCALEWAY_EMAIL_PROJECT_ID`, secret key, and `fr-par` region;
- `NVBES_EMAIL_QUEUE_URL`, endpoint, region, and publish-only access/secret key;
- `NVBES_EMAIL_RUNTIME_ROLE`, set to `ingress` or `dispatch` by Terraform;
- `NVBES_EMAIL_SNS_TOPIC_ARN`;
- `NVBES_EMAIL_SNS_CA_BUNDLE_PEM` containing the pinned Scaleway SNS trust
  chain. A read-only file path (`NVBES_EMAIL_SNS_CA_BUNDLE_PATH`) remains supported outside Serverless Containers.
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

Scrape `GET /metrics` only through the private observability network or with the
dedicated internal bearer token. The runtime emits command outcomes, provider latency,
webhook outcomes, signature failures, expirations, suppressions, and retention activity.
Alert on queue depth, oldest-message age, trigger failures, and DLQ depth using Scaleway MNQ metrics,
because querying PostgreSQL for those gauges would defeat idle scaling.

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
`infrastructure/environments/email-production/email-provider.tf` (and `infrastructure/stacks/email/production/email-delivery.tf`) is the sole
writer.
Terraform registers `notify.nvbes.eu` in Scaleway TEM, publishes the exact SPF,
DKIM, DMARC, and blackhole MX values returned by TEM, requests validation,
creates the SNS topic and HTTPS subscription, then binds the TEM webhook. Do
not copy example record values into production and do not mutate these
resources through either provider console, `scw`, or Wrangler. Import any
pre-existing resource into the production state before applying the stack.
The apex `nvbes.eu` remains separate so Cloudflare Email Routing can later
receive human mail without replacing the transactional subdomain MX records.

1. From `main`, run the GitHub Action `container release` with application
   `email-worker`. It publishes, scans, attests, and signs only the commit image.
2. Run the GitHub Action `deploy email`. The protected `production-email`
   Environment requires operator approval and exposes only email-scoped
   credentials.
3. The workflow resolves the GHCR commit tag to an immutable digest, verifies
   its release certificate, and mirrors only the verified `linux/amd64` image
   into the private Terraform-owned Scaleway Container Registry. It signs and
   verifies that private copy before Terraform receives its digest.
4. Terraform initializes only `production/email/terraform.tfstate`, applies a
   reviewed migration-foundation plan, and creates or updates the migration job.
5. The workflow starts that job through the Scaleway Serverless Jobs API and
   requires `succeeded`. The job receives only the migration DSN from Secret
   Manager and runs `nvbes-email-worker migrate`; normal container startup never
   runs migrations.
6. Terraform replans and applies the complete isolated email stack. This creates
   or updates the SQS queue, DLQ, scale-to-zero ingress/dispatcher, triggers,
   TEM, SNS, and DNS without touching the shared `DB-PRO2-M` or another product.
7. The workflow confirms `/health/live`. Then send one short-lived test command
   and confirm SQS consumption, TEM acceptance, and the signed webhook
   transition before enabling product relays.

Never run a production apply with local state. The
`infrastructure/environments/email-production/README.md` file defines the
protected GitHub Environment, backend, and credential contract. Import any
pre-existing email resource into the isolated state before the first workflow
run; Terraform provider credentials must come from GitHub Environment secrets.

## Deadline incidents

A credential email must use the authoritative credential expiry as
`deliver_before`. The dispatcher checks database time before claiming and
again immediately before the provider call. When the deadline prevents the
next attempt, the message becomes terminal `expired`; it is not retried.

Provider-accepted messages never transition to `expired` later. Their embedded
credential may expire, but the delivery itself already occurred.

## Provider incidents

During a TEM outage, commands remain durable in PostgreSQL and SQS. Transient or
ambiguous failures return HTTP 503 to the trigger and retry on the 60-second SQS
visibility window, up to four provider attempts and never beyond
`deliver_before`. Investigate `email_delivery_duration_seconds`,
`email_messages_total`, trigger errors, and DLQ depth. Do not bypass the worker
or send directly through SMTP or the provider API.

The database uses `min_cpu=0` and can become idle after traffic stops. A first
query after an idle period may therefore be slower; the SQLx acquisition budget
is 15 seconds. Do not add keepalive queries, database-backed probes, or polling,
as any of them would prevent the intended idle behavior.

Scaleway limits one Serverless SQL request and the prepared statements held by
one client connection to 1,024 KiB. `email-worker` keeps a deliberate margin:
the gRPC transport rejects decoded messages above 256 KiB and the application
rejects an encoded `SubmitEmailRequest` above 192 KiB before encryption or any
database call. URLs are limited to 4,096 bytes, email addresses to 320 bytes,
and human-readable template fields to 200 bytes. Do not raise these limits
without re-auditing the complete PostgreSQL bind request and SQLx statement
cache against Scaleway's current technical limits.

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
after `NVBES_EMAIL_LEDGER_RETENTION_DAYS`. A daily Scaleway cron trigger performs
retention and stale-deadline cleanup; no resident maintenance loop exists.

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
