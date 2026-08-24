# Production Trust/Risk stack

This Terraform root owns the independent production Trust/Risk engine:

- a dedicated Serverless SQL database with separate runtime and migration identities;
- a private registry for immutable verified images;
- an explicit Serverless Job for SQLx migrations;
- one scale-to-zero public `h2c` container exposing authenticated gRPC APIs and health endpoints.

Both the container and Serverless SQL database have a zero minimum. This is an
explicit FinOps policy: cold-start latency after idle periods is accepted in
exchange for avoiding continuously running compute.

The remote state key is fixed to `production/trust-risk/terraform.tfstate`.
Critical database and registry resources use `prevent_destroy`.

## GitHub environment

Create a protected environment named `production-trust-risk` with variables
`SCW_PROJECT_ID`, `SCW_PRIVATE_NETWORK_ID` and `TERRAFORM_STATE_BUCKET`.
It requires these secrets:

- `SCW_ACCESS_KEY` and `SCW_SECRET_KEY` for the least-privilege deployment identity;
- `TRUST_RISK_TERRAFORM_STATE_ACCESS_KEY` and `TRUST_RISK_TERRAFORM_STATE_SECRET_KEY`;
- `TRUST_RISK_PRODUCER_POLICIES`, a non-empty JSON array;
- `TRUST_RISK_OPERATOR_TOKENS`, a non-empty JSON array;
- `TRUST_RISK_METRICS_TOKEN`, at least 32 characters.

Run `deploy trust risk` manually from `main`. It builds and scans the image,
mirrors its verified digest into Scaleway, migrates the database, validates the
production configuration, applies the runtime plan, and probes liveness and
readiness. Never commit real tokens, credentials, plans or Terraform state.
