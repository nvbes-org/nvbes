# Global email delivery runbook

## Runtime topology

Run `nvbes-email-worker` as an always-on container. It owns three concurrent
roles in one process:

- private gRPC ingestion on `3041`;
- public HTTP webhook and probes on `3040`;
- PostgreSQL-backed dispatch loop.

Do not deploy it as a scale-to-zero request-only job: the dispatcher must make
progress without an inbound request. Product services use gRPC only. HTTP is
reserved for Scaleway Topics and Events and health probes.

## Health ownership

- The container runtime calls `GET /health/live`. This is shallow and must not
  depend on PostgreSQL or Scaleway TEM.
- The private Scaleway Load Balancer calls `GET /health/ready`. It requires a
  reachable, migrated database and a current dispatcher heartbeat.
- Internal gRPC diagnostics may call `grpc.health.v1.Health` on `3041`.
- Scaleway probes never call the gRPC health service.

The TEM provider is deliberately absent from readiness. A TEM outage must
accumulate durable commands instead of evicting every ingress instance.

## Required production configuration

Configure the runtime with:

- `NVBES_EMAIL_DATABASE_URL`;
- `NVBES_EMAIL_GRPC_AUTH_TOKEN` and `NVBES_EMAIL_ALLOWED_PRODUCERS`;
- `NVBES_EMAIL_DATA_ENCRYPTION_KEY` and `NVBES_EMAIL_RECIPIENT_HMAC_KEY`;
- `NVBES_EMAIL_FROM_EMAIL`, sender name, and optional reply-to;
- `NVBES_SCALEWAY_EMAIL_PROJECT_ID`, secret key, and `fr-par` region;
- `NVBES_EMAIL_SNS_TOPIC_ARN`;
- `NVBES_EMAIL_SNS_CA_BUNDLE_PATH` containing the pinned Scaleway SNS trust
  chain.

Mount secrets read-only. Product services receive only the gRPC endpoint and
internal authentication token; they never receive provider or encryption
credentials.

## Deployment order

1. Apply the email database migration.
2. Verify the TEM sender domain and Topics and Events topic.
3. Start the container and wait for `/health/live` then `/health/ready`.
4. Subscribe the signed webhook endpoint
   `/webhooks/scaleway/topics-and-events` and verify confirmation.
5. Send one short-lived test command and confirm provider acceptance plus the
   webhook transition.
6. Enable product relays.

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
