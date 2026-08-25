# Email production readiness — 25 August 2026

## Decision

**NO-GO for a new Email apply or user traffic.**

The production Email stack already exists and is healthy at the shallow
liveness level, but its live configuration predates the V1 FinOps contract now
merged locally. No Terraform apply, provider mutation, email delivery or restore
operation was performed during this audit.

## Scope and authority

The audit covers the repository, GitHub Actions configuration, public DNS,
read-only Scaleway inventory, current provider consumption and managed database
backups. It does not assert the value of secrets, organization-level GitHub
configuration that the current token cannot read, or the correctness of a
Terraform remote state that has not yet been refreshed in a plan-only run.

Provider consumption and taxes are the cost authority. Repository contracts are
the desired configuration authority. A live/read-model mismatch remains a
blocker until a reviewed plan reconciles it.

## Verified evidence

| Area | Evidence | Result |
| --- | --- | --- |
| Last Email deployment | GitHub Actions run `32683865487`, commit `8cf16c69`, 24 August 2026 | Successful |
| Public liveness | Production ingress `/health/live` | `alive` |
| Sender domain | Scaleway TEM `notify.nvbes.eu` | `checked`, reputation 100 |
| Public DNS | SPF, DKIM, DMARC and blackhole MX resolve | Present |
| Database | `nvbes-prod-email`, PostgreSQL 16 | Ready, idle, CPU 0/1 |
| Backups | Managed backups from 24 and 25 August | Ready, seven-day expiry |
| Runtime image | Private immutable registry image by digest | Present |
| Local Email tests | 58 library + 2 adapter + 37 worker tests | 97 passed |
| Container contract | Nx `email-worker:container-contract` | 3 passed |
| Terraform | init without backend and validate | Valid |
| FinOps gate | `pnpm check:finops` | 115 passed |
| Workflow security | targeted CI/CD security checker | Passed |

No transactional message has yet been recorded by TEM. This audit therefore
does not prove command persistence, SQS consumption, provider acceptance or the
signed webhook transition.

## Current FinOps observation

For the Scaleway project `nvbes`, provider consumption for August at audit time
was **0.55 EUR before tax** and the reported 20% tax was **0.11 EUR**, for a
current total of **0.66 EUR TTC**. The history begins only around the recent
Email and Trust/Risk deployments, so it is not sufficient to claim a reliable
30-day forecast.

No Scaleway budget is configured. The repository target remains 20 EUR TTC and
the absolute project limit remains 30 EUR TTC. Provider alerts at 15, 20, 25,
28 and 30 EUR remain mandatory informational controls before the next apply.

The current Scaleway catalog prices Serverless Containers at 0.00001 EUR per
vCPU-second and 0.000002 EUR per GB-second, before tax. At the configured 0.28
vCPU and 0.5 GB per Email role, two Email containers active continuously for a
30-day month would cost approximately **19.70 EUR before tax / 23.64 EUR TTC**.
The configured 0.56 vCPU and 1 GB Trust/Risk container has the same continuous
compute cost. Together they would reach approximately **47.28 EUR TTC**, before
databases, storage, messaging, registry, email and observability.

Consequently, `max_scale = 1` is a necessary capacity bound but is not a hard
30 EUR spending guarantee. Internal synthetic tests must be time-bounded and
monitored. Public traffic requires an independently reviewed quota or automatic
economic shutdown path before it can be considered.

## Blocking findings

### 1. Live capacity exceeds the repository contract

Both live Email containers have `min_scale = 0` but `max_scale = 10`. The
current Terraform contract requires `max_scale = 1`. The live Trust/Risk
container also reports `max_scale = 10`, which matters because the 30 EUR limit
covers the entire project.

No new production apply or traffic opening is allowed until all targeted live
runtimes are capped at one and a post-change read verifies the result.
The cap permits internal synthetic validation only; it does not close the
continuous-usage cost risk described above.

### 2. The audited code is not published

Local `main` is forty commits ahead of `origin/main`. GitHub therefore cannot
run the FinOps-enforced workflow or apply the corrected capacity bounds yet.
Publishing is a separate external change and requires an explicit integration
decision.

### 3. GitHub production approval is not enforced

The `production-email` environment exists but has no protection rule or
deployment branch policy. Branch protection and repository rulesets are not
available for this private repository on the current GitHub plan.

Until a paid control exists, the free solo-operator fallback must be encoded in
the workflow: manual dispatch from `main`, an exact confirmation phrase and the
expected full commit SHA. Environment-scoped credentials remain mandatory.
The audited branch now implements this repository-side contract, but it remains
inactive until published and until both environment variables are configured.

### 4. Visible GitHub configuration is incomplete

The repository-level `CLOUDFLARE_API_TOKEN` and variables including
`SCW_PROJECT_ID` are present. At the visible repository and environment scopes,
the following required values are absent:

- secrets: `EMAIL_SENTRY_DSN`, `GRAFANA_SERVICE_ACCOUNT_TOKEN`;
- variables: `GRAFANA_EMAIL_CONTACT_POINT`,
  `GRAFANA_PROMETHEUS_DATASOURCE_UID`, `GRAFANA_URL`,
  `EMAIL_DEPLOY_CONFIRMATION`, `EMAIL_DEPLOY_APPROVED_SHA`.

Organization-level values could not be enumerated with the current GitHub token.
Their existence must be verified without exposing their contents before these
items can be closed.

### 5. Observability is not proven on the deployed image

Anonymous `GET /metrics` currently returns HTTP 404. The current contract
expects a bearer-protected metrics endpoint, an authenticated scrape, Grafana
alerts and a successful Sentry smoke event. None is proven for the deployed
Email image by this audit.

### 6. Restore and rollback are unproven

Managed backups exist, but no isolated restore result records RPO, RTO or data
integrity. The deployment workflow has no executed rollback evidence. A backup
existing is not proof that recovery works.

### 7. End-to-end delivery is unproven

TEM reports zero messages. A single synthetic transactional command must prove
durable acceptance, queue consumption, provider acceptance and signed webhook
processing before any product is connected.

## Remediation sequence

1. Keep product producers disabled and do not run the deploy workflow.
2. Publish the audited `main` only after reviewing its forty local commits.
3. Configure the free, fail-closed manual confirmation and expected-SHA values
   required by the audited Email deployment workflow.
4. Verify or configure all missing GitHub values without printing secrets.
5. Configure Scaleway informational budget alerts at 15, 20, 25, 28 and 30 EUR.
6. Run a remote-state, plan-only Terraform refresh; reject any unexpected
   replacement, deletion, ownership change or capacity increase.
7. Reconcile Email and Trust/Risk live capacity to `min = 0`, `max = 1` through
   the reviewed infrastructure path.
8. Define and test the time or request budget that turns off non-essential live
   compute before the project can cross 30 EUR TTC.
9. Verify liveness, authenticated metrics, anonymous metrics denial, Sentry and
   Grafana alerts.
10. Send one short-lived synthetic email to an operator-controlled address and
   verify its complete event history.
11. Restore the latest backup into an isolated target, measure RPO/RTO, run
    integrity checks and record cleanup.
12. Re-read provider consumption and issue a new dated readiness decision.

## Go criteria

The decision may become **GO for internal synthetic traffic only** when every
blocking finding is closed, the reviewed Terraform plan contains no unexpected
destructive action, the observed and projected TTC costs remain below the V1
gates, and rollback plus restoration evidence is attached to the superseding
readiness record. Public or invited accounts remain out of scope.
