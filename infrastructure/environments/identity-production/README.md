# Production Identity

This isolated Terraform root owns the closed production Identity foundation:

- a private immutable container registry;
- a scale-to-zero container (`min_scale = 0`, `max_scale = 1`);
- a dedicated scale-to-zero Serverless SQL database;
- separate data-only runtime and DDL-capable migrator identities;
- a protected migration secret and explicit migration job.

Its remote state key is fixed to `production/identity/terraform.tfstate` and is
already reserved by the production bootstrap. No other stack may own these
resources.

The HTTP endpoint intentionally exposes only shallow liveness, database-backed
readiness and bearer-protected metrics. Registration, login, invited accounts and
product traffic remain closed. `privacy = "public"` is required for Scaleway
health probes and does not imply a public authentication API.

## Protected GitHub environment

Create `production-identity` restricted to `main`, without administrator bypass,
and provide distinct least-privilege credentials:

- secrets: `SCW_ACCESS_KEY`, `SCW_SECRET_KEY`,
  `IDENTITY_TERRAFORM_STATE_ACCESS_KEY`,
  `IDENTITY_TERRAFORM_STATE_SECRET_KEY`, `IDENTITY_MFA_ENCRYPTION_KEY`,
  `IDENTITY_METRICS_TOKEN`, `IDENTITY_SYNTHETIC_PASSWORD`,
  `IDENTITY_SYNTHETIC_RECOVERED_PASSWORD`, `IDENTITY_SENTRY_DSN`,
  `GRAFANA_OTLP_AUTHORIZATION_HEADER`;
- variables: `SCW_PROJECT_ID`, `SCW_ORGANIZATION_ID`, `SCW_REGION`, `SCW_ZONE`,
  `TERRAFORM_STATE_BUCKET`, `GRAFANA_OTLP_ENDPOINT`,
  `IDENTITY_MFA_KEY_VERSION`, `IDENTITY_SENTRY_TRACES_SAMPLE_RATE`;
- per-deployment approvals: `IDENTITY_DEPLOY_CONFIRMATION` equal to
  `deploy-identity-production` and `IDENTITY_DEPLOY_APPROVED_SHA` equal to the
  exact `main` commit.

The deployment workflow must build and scan the image, verify its signature,
materialize database identities, execute the migration job, apply the runtime
plan, run the non-delivering synthetic authentication/recovery job and probe
liveness/readiness. Each run uses a unique `.invalid` address and emits only its
principal UUID and bounded audit counts. Rollback uses the previously captured image
digest; migrations are additive and remain compatible with that image.
