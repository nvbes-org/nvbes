# Cutover Evidence Packet

## Status

- readiness: no-go
- repository_ready: true
- live_blocking_items: 6
- completion: no-go
- backlog_open_tasks: 25
- backlog_blocking_items: 58
- decision: no-go

## Rules

- `repository_ready` must equal every requirement repository readiness.
- `live_blocking_items` must equal requirements blocked by missing repository artifacts, pending phase/gate state or final reconciliation.
- Requirement blocking reasons must match current repository, phase, gate and final reconciliation state.
- `decision: go` requires readiness `go`, repository readiness, completion `go`, zero backlog tasks and zero live blockers.
- `decision: go` requires every strict command segment to be covered by a live evidence preparation command.
- Requirement rows must match the static cutover packet contract exactly.
- Generation provenance must identify write and strict check commands.

## Evidence Requirements

| ID | Scope | Repository Ready | Current Phase | Current Gate | Blocking Reasons | Uncovered Strict Segments | Live Evidence |
|---|---|---:|---|---|---|---|---|
| g4-frontend-signoff | G4 Frontends | true | not applicable | pending/no-go | gate pending/no-go | none | critical Playwright journeys and WCAG AA sign-off |
| g5-infra-signoff | G5 Infra | true | not applicable | pending/no-go | gate pending/no-go | none | staging rebuild, backup restore and timed rollback proof |
| p11-rehearsals | P11/G6 Repetitions migration | true | pending/no-go | pending/no-go | phase pending/no-go, gate pending/no-go | none | three rehearsal runs and two stable reconciliations |
| p12-cutover | P12/G7 Big Bang cutover | true | pending/no-go | pending/no-go | phase pending/no-go, gate pending/no-go | none | production cutover journal, final reconciliation, smoke and SLO proof |
| p13-decommission | P13/G8 Decommission | true | pending/no-go | pending/no-go | phase pending/no-go, gate pending/no-go | none | legacy runtime removed, secrets revoked and post-migration audit approved |
| final-reconciliation | Production reconciliation | true | not applicable | not applicable | production reconciliation not attached | none | executed production reconciliation JSON replacing the template no-go report |

## Strict Commands

- g4-frontend-signoff: `pnpm check:migration-frontend-experience && pnpm check:web && pnpm check:migration-smoke-tests -- --strict`
- g5-infra-signoff: `pnpm check:migration-infra-deploy && pnpm check:migration-backup-restore -- --strict && pnpm check:migration-rollback-report -- --strict`
- p11-rehearsals: `pnpm check:migration-rehearsals -- --strict && node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json && pnpm check:migration-rejects -- --strict`
- p12-cutover: `pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json`
- p13-decommission: `pnpm check:migration-postcutover`
- final-reconciliation: `node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json`

## Preparation Commands

### g4-frontend-signoff

```bash
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-frontend-signoff-local-1 --type frontend_signoff --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-frontend-signoff-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-frontend-experience' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-frontend-signoff-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-frontend-signoff-local-1.json
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-web-check-local-1 --type web_check --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-web-check-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:web' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-web-check-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-web-check-local-1.json
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-smoke-test-local-1 --type smoke_test --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-smoke-test-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-smoke-tests -- --strict' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-smoke-test-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-smoke-test-local-1.json
```

### g5-infra-signoff

```bash
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-infra-deploy-staging-1 --type infra_deploy --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-infra-deploy-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-infra-deploy' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-infra-deploy-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-infra-deploy-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-infra-restore-staging-1 --type infra_restore --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-infra-restore-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-backup-restore -- --strict' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-infra-restore-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-infra-restore-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-rollback-staging-1 --type rollback --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-rollback-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rollback-report -- --strict' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-rollback-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-rollback-staging-1.json
```

### p11-rehearsals

```bash
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-local-1 --type rehearsal --env local --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-local-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-local-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-staging-2 --type rehearsal --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-staging-2.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-staging-2 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-staging-2.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-production-3 --type rehearsal --env production --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-production-3.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-production-3 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-production-3.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reconciliation-staging-1 --type reconciliation --env staging --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reconciliation-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reconciliation-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reconciliation-production-2 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reconciliation-production-2 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reconciliation-production-2.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rollback-staging-1 --type rollback --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rollback-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rollback-report -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rollback-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rollback-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reject-review-staging-1 --type reject_review --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-reject-review-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rejects -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reject-review-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reject-review-staging-1.json
```

### p12-cutover

```bash
node tools/migration/live-evidence-prepare.mjs --id p12-cutover-cutover-production-1 --type cutover --env production --owner '<owner>' --source-artifact artifact://migration/p12-cutover-cutover-production-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p12-cutover-cutover-production-1 --packet-requirement p12-cutover --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p12-cutover-cutover-production-1.json
node tools/migration/live-evidence-prepare.mjs --id p12-cutover-reconciliation-production-1 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p12-cutover-reconciliation-production-1 --packet-requirement p12-cutover --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p12-cutover-reconciliation-production-1.json
```

### p13-decommission

```bash
node tools/migration/live-evidence-prepare.mjs --id p13-decommission-decommission-production-1 --type decommission --env production --owner '<owner>' --source-artifact artifact://migration/p13-decommission-decommission-production-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-postcutover' --result passed --decision go --immutable-reference artifact://migration/p13-decommission-decommission-production-1 --packet-requirement p13-decommission --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p13-decommission-decommission-production-1.json
```

### final-reconciliation

```bash
node tools/migration/live-evidence-prepare.mjs --id final-reconciliation-reconciliation-production-1 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/final-reconciliation-reconciliation-production-1 --packet-requirement final-reconciliation --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/final-reconciliation-reconciliation-production-1.json
```


## Decision

No-go remains until live rehearsal, cutover, reconciliation and decommission evidence replaces the pending templates.
