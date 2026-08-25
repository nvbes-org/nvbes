# Production FinOps contract

`production-budget.json` is the executable tax-inclusive monthly budget for the
nvbes V1 production foundation.

The target is 20 EUR and the hard limit is 30 EUR, including compute, databases,
email, storage, observability, domains, taxes and every recurring provider.

## Change procedure

1. Record the latest tax-inclusive invoice or provider estimate.
2. Update the relevant category without raising the 20 EUR target or 30 EUR hard
   limit.
3. Run `pnpm check:finops`.
4. Include the measured reason and affected cost unit in the commit message or
   pull-request description.

Changing the approved ceilings requires a new design decision. Provider alerts
do not replace repository limits.

## Degradation stages

- `normal`: all budgeted V1 work is available.
- `disable_non_essential`: optional processing is disabled from 25 EUR.
- `freeze_cost_creation`: new cost-creating operations are frozen from 28 EUR.
- `essential_only`: only recovery, critical email and durable provider webhook
  ingestion remain from 30 EUR.

Platform Operations will consume the shared Rust stage selector when live spend
collection is implemented. Until that runtime exists, literal Terraform maxima
and provider-side alerts are the automatic outer guardrails.
