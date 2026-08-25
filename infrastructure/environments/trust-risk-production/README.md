# Production Trust/Risk stack

This Terraform root owns the independent production Trust/Risk engine:

- a dedicated Serverless SQL database with separate runtime and migration identities;
- a private registry for immutable verified images;
- an explicit Serverless Job for SQLx migrations;
- one scale-to-zero public `h2c` container exposing authenticated gRPC APIs and health endpoints.
- Sentry error reporting keyed by the immutable release digest;
- direct OTLP traces and business metrics to Grafana Cloud, plus a managed dashboard and alert rules.

Both the container and Serverless SQL database have a zero minimum. This is an
explicit FinOps policy: cold-start latency after idle periods is accepted in
exchange for avoiding continuously running compute.

The remote state key is fixed to `production/trust-risk/terraform.tfstate`.
Critical database and registry resources use `prevent_destroy`.

## GitHub environment

Create a protected environment named `production-trust-risk` with variables
`SCW_ORGANIZATION_ID`, `SCW_PROJECT_ID`, `SCW_PRIVATE_NETWORK_ID` and
`TERRAFORM_STATE_BUCKET`. Configure `GRAFANA_URL`, `GRAFANA_OTLP_ENDPOINT`,
`GRAFANA_PROMETHEUS_DATASOURCE_UID`, `GRAFANA_TEMPO_DATASOURCE_UID` and
`GRAFANA_TRUST_RISK_CONTACT_POINT` from the production Grafana Cloud stack.
It requires these secrets:

- `SCW_ACCESS_KEY` and `SCW_SECRET_KEY` for the least-privilege deployment identity;
- `TRUST_RISK_TERRAFORM_STATE_ACCESS_KEY` and `TRUST_RISK_TERRAFORM_STATE_SECRET_KEY`;
- `TRUST_RISK_PRODUCER_POLICIES`, a non-empty JSON array;
- `TRUST_RISK_OPERATOR_TOKENS`, a non-empty JSON array;
- `TRUST_RISK_METRICS_TOKEN`, at least 32 characters.
- `TRUST_RISK_SENTRY_DSN`, scoped to the Sentry Trust/Risk project;
- `GRAFANA_SERVICE_ACCOUNT_TOKEN`, limited to dashboard and alert provisioning;
- `GRAFANA_OTLP_AUTHORIZATION_HEADER`, a precomputed Basic header using an
  access-policy token limited to metrics and traces ingestion.

Trust/Risk exports directly to Grafana Cloud because a permanent Alloy process
would violate the scale-to-zero FinOps policy. The exporter exists only inside
request-driven Serverless Container instances and flushes metrics on graceful
shutdown.

Run `deploy trust risk` manually from `main`. It builds and scans the image,
mirrors its verified digest into Scaleway, migrates the database, validates the
production configuration, applies the runtime plan, and probes liveness and
readiness. Never commit real tokens, credentials, plans or Terraform state.
