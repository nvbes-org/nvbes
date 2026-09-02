# Email production readiness — 2 September 2026

## Decision

**GO for bounded internal synthetic traffic only.**

This record supersedes the NO-GO decision in
[`email-production-readiness-2026-08-25.md`](email-production-readiness-2026-08-25.md).
It does not authorize product, invited-account or public user traffic. Every
validation window remains operator-controlled, temporary and reversible.

## Scope and evidence authorities

The decision covers the reusable V1 Email foundation: authenticated ingress,
SQS dispatch, Scaleway TEM delivery, signed SNS callbacks, ledger persistence,
metrics, Sentry, Grafana alerting, database recovery, image rollback and FinOps
controls.

The repository and remote state key `production/email/terraform.tfstate` are
the desired-configuration authorities. GitHub Actions, Scaleway, Grafana and
the restored database are the runtime evidence authorities. The repository
budget contract and Scaleway Billing are the cost authorities.

## Published release and protected deployment

The signed canonical release is
`c3e0e7a0692dd7a90ff08647fd518c886810e980`. Protected GitHub Actions run
[`33628770780`](https://github.com/nvbes-org/nvbes/actions/runs/33628770780)
completed successfully with the exact approved SHA.

The run proved:

- the CI, FinOps, Terraform, Rust and container contract gates;
- an immutable linux/amd64 image build, vulnerability scan, CycloneDX SBOM and
  keyless Cosign signature;
- mirroring and signature verification in the private Scaleway registry;
- reviewed saved Terraform plans followed by application of those exact plans;
- database migrations, Terraform-owned SNS subscription and runtime health;
- a bounded synthetic window followed by the unconditional economic shutdown.

Canonical private image:

`rg.fr-par.scw.cloud/nvbes-prod-email-worker/nvbes-email-worker@sha256:b7311203c242fa3066d597c4e8caf76250a198f52d847bbc5afbdb653314fd9d`

GitHub repository and `production-email` environment variables both bind the
release approval to the full canonical SHA. Required secrets remain scoped to
the protected environment and are demonstrated by successful authenticated
operations without being reproduced here. `EMAIL_SYNTHETIC_SMOKE_ENABLED` is
now `false` at both scopes.

## Terraform and runtime convergence

The protected deployment initialized only the isolated Email state key. It
reviewed and applied separate saved plans for the private registry, migration
foundation, runtime foundation and complete Email runtime. The deployed image
digest, Serverless SQL database, two scale-to-zero containers, queues, TEM
domain, SNS topic and single HTTPS subscription are managed by that state.

The post-validation shutdown plan recorded `0 to add, 2 to change, 2 to
destroy`. Its exact apply made ingress private and destroyed both temporary
container triggers. The workflow then read the live Scaleway API and required
private ingress plus zero dispatch triggers.

Final Email execution bounds are `min_scale = 0`, `max_scale = 1`, private
ingress and dispatch, and no unattended queue or retention trigger.

## Complete synthetic delivery

Synthetic run `deploy-33628770780-1` exercised the real authenticated gRPC,
database, queue, dispatch, Scaleway TEM and signed webhook path. The deployment
gate required a non-empty message ID, one to five attempts, final state
`delivered` and at least one processed provider event.

Recovery run
[`33632672089`](https://github.com/nvbes-org/nvbes/actions/runs/33632672089)
then queried the live ledger and recorded exactly:

```json
{
  "messages": 1,
  "state": "delivered",
  "attempts": 1,
  "provider_events": 2,
  "can_select": true,
  "can_insert": true,
  "can_update": true
}
```

## Observability

Deployment run `33628770780` proved that anonymous `/metrics` access returns
HTTP 401 and that the bearer-protected endpoint exposes the queue depth and
oldest-message-age metrics consumed by Grafana Alloy.

The same protected release ran the dedicated Sentry error-reporting smoke with
release `c3e0e7a0692dd7a90ff08647fd518c886810e980` and trace sample rate `0.1`.
The gate required the event to be configured, sent and flushed.

Recovery run `33632672089` read Grafana directly and proved dashboard
`nvbes-email-production-v1` plus exactly three active, unpaused Email rules:

- `nvbes-email-queue-stale`;
- `nvbes-email-provider-error-rate`;
- `nvbes-email-complaint-rate`.

It also proved one confirmed HTTPS SNS subscription to the exact Email webhook.

## Fresh restore and reversible rollback

Recovery run `33632672089` selected ready backup
`184103e9-42f2-4e10-8856-0f3ec7e7d19a`, created at
`2026-09-02T01:00:00.843181Z`. It created isolated database
`ba89e65d-61fb-4795-8a75-e3bf3b43b1b0`, started PostgreSQL 16 and validated:

```json
{
  "migrations": 4,
  "tables": 5,
  "messages": 4,
  "attempts": 4,
  "provider_events": 6,
  "constraints_valid": true
}
```

The measured create-to-integrity interval was approximately 85 seconds. The
workflow deleted the isolated database in its unconditional cleanup step.

With both runtimes still private and scale-to-zero, the same run applied the
previous immutable image
`sha256:c4a2d016c3793bd93bad41c44fee9dbc2a4690d044b2542aed76f0b6ed0fb45b`
to both containers and verified it through the live API. It then applied the
canonical image recorded above and verified both containers again. Each
direction used a saved Terraform plan and changed exactly two resources.

## FinOps and economic shutdown

Operator audit records Scaleway budget
`d0b76811-3993-40e6-aeaa-5121bb044285` as enabled at **30 EUR**, with five
notifications at 50%, 66%, 83%, 93% and 100%. These correspond to 15.00,
19.80, 24.90, 27.90 and 30.00 EUR and conservatively cover the repository
thresholds at 15, 20, 25, 28 and 30 EUR.

The protected Email credential receives HTTP 403 from the Scaleway Billing API
and therefore cannot independently re-read this configuration. This is an
intentional least-privilege boundary: the deployment credential retains the
project permissions required by Email without receiving account-level Billing
access. The provider alert configuration is consequently operator-audited,
while the repository contract and tested runtime shutdown are executable
evidence.

Alerts are informational rather than a provider-side hard cap. The tested
automatic protection for this scope is the unconditional Email shutdown:

- synthetic enablement is false in GitHub;
- ingress and dispatch are private;
- both temporary triggers are absent;
- minimum scale is zero and maximum scale is one;
- every validation run closes the window even when an earlier step fails.

Consequently, Email synthetic traffic cannot continue unattended toward or
beyond the 30 EUR TTC project ceiling. This does not prove that unrelated
resources in the shared Scaleway project can never spend beyond the ceiling;
the solo operator must still act on the staged alerts for those resources.

Final protected audit
[`33635401102`](https://github.com/nvbes-org/nvbes/actions/runs/33635401102)
confirmed that the canonical database and TEM keys remain present, removed the
three obsolete recovery IAM keys and the temporary synthetic SQS credential,
restored the canonical image after another rollback exercise, and deleted its
automatically provisioned recovery database.

## Residual restrictions

- Only an operator may open a synthetic validation window.
- One diagnostic message at a time is permitted.
- Product producers, invited accounts and public traffic remain disabled.
- Budget alerts require operator action outside the tested Email shutdown.
- Any image, secret, Terraform, IAM, DNS, TEM, SNS, SQS, database, metrics,
  Sentry or Grafana change requires a new bounded validation and readiness
  record.

## Final readiness statement

The blockers recorded on 25 August are closed for **internal synthetic Email
traffic**: published code, protected configuration, remote Terraform plans,
metrics, Sentry, Grafana alerts, full delivery, fresh restore, real image
rollback and bounded economic shutdown all have direct evidence. The
authorization remains narrow, measured and reversible; it is not a public
production launch decision.
