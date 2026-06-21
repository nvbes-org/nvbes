# Migration Execution Backlog

## Status

- open_tasks: 25
- blocking_items: 58
- readiness: no-go
- completion: incomplete
- live_evidence: no-go
- live_evidence_missing_requirements: 6
- live_evidence_missing_items: 24

## Rules

- `open_tasks` must equal generated tasks that are not complete.
- `blocking_items` must equal the sum of task blocking counts.
- Blocked tasks require at least one blocking item.
- Complete tasks must have zero blocking items.
- Task IDs must be unique and every task must carry owner and proof.
- Markdown task rows must expose owner, source, next action, status, blocking count, blocking details and proof.
- Proof commands must reference existing package scripts or migration tools.
- Live evidence tasks must list generated preparation command templates.
- Generation provenance must identify the write and strict check commands.

## Tasks

| ID | Owner | Source | Next action | Status | Blocking | Blocking details | Proof |
|---|---|---|---|---|---:|---|---|
| attach-live-evidence-final-reconciliation | Data lead | live-evidence:Production reconciliation | attach accepted live evidence for reconciliation:0/1, reconciliation@production | blocked | 2 | `reconciliation:0/1`<br>`reconciliation@production` | `node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json && pnpm check:migration-live-evidence-instances -- --strict` |
| attach-live-evidence-g4-frontend-signoff | Product leads | live-evidence:G4 Frontends | attach accepted live evidence for frontend_signoff:0/1, web_check:0/1, smoke_test:0/1 | blocked | 3 | `frontend_signoff:0/1`<br>`web_check:0/1`<br>`smoke_test:0/1` | `pnpm check:migration-frontend-experience && pnpm check:web && pnpm check:migration-smoke-tests -- --strict && pnpm check:migration-live-evidence-instances -- --strict` |
| attach-live-evidence-g5-infra-signoff | Infra lead | live-evidence:G5 Infra | attach accepted live evidence for infra_deploy:0/1, infra_deploy@staging, infra_restore:0/1, infra_restore@staging, rollback:0/1, rollback@staging | blocked | 6 | `infra_deploy:0/1`<br>`infra_deploy@staging`<br>`infra_restore:0/1`<br>`infra_restore@staging`<br>`rollback:0/1`<br>`rollback@staging` | `pnpm check:migration-infra-deploy && pnpm check:migration-backup-restore -- --strict && pnpm check:migration-rollback-report -- --strict && pnpm check:migration-live-evidence-instances -- --strict` |
| attach-live-evidence-p11-rehearsals | Data lead | live-evidence:P11/G6 Repetitions migration | attach accepted live evidence for rehearsal:0/3, rehearsal@local, rehearsal@staging, rehearsal@production, reconciliation:0/2, rollback:0/1, reject_review:0/1 | blocked | 7 | `rehearsal:0/3`<br>`rehearsal@local`<br>`rehearsal@staging`<br>`rehearsal@production`<br>`reconciliation:0/2`<br>`rollback:0/1`<br>`reject_review:0/1` | `pnpm check:migration-rehearsals -- --strict && node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json && pnpm check:migration-rejects -- --strict && pnpm check:migration-live-evidence-instances -- --strict` |
| attach-live-evidence-p12-cutover | Migration lead | live-evidence:P12/G7 Big Bang cutover | attach accepted live evidence for cutover:0/1, cutover@production, reconciliation:0/1, reconciliation@production | blocked | 4 | `cutover:0/1`<br>`cutover@production`<br>`reconciliation:0/1`<br>`reconciliation@production` | `pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json && pnpm check:migration-live-evidence-instances -- --strict` |
| attach-live-evidence-p13-decommission | Infra lead | live-evidence:P13/G8 Decommission | attach accepted live evidence for decommission:0/1, decommission@production | blocked | 2 | `decommission:0/1`<br>`decommission@production` | `pnpm check:migration-postcutover && pnpm check:migration-live-evidence-instances -- --strict` |
| complete-backup-rollback | Migration lead | blueprint:Criteres de Reussite | les backups et rollbacks sont testes | blocked | 1 | `les backups et rollbacks sont testes` | `pnpm check:migration-backup-restore -- --strict && pnpm check:migration-rollback-report -- --strict` |
| complete-critical-journeys | Migration lead | blueprint:Criteres de Reussite | tous les parcours critiques ont une implementation nouvelle | blocked | 1 | `tous les parcours critiques ont une implementation nouvelle` | `pnpm check:migration-parity -- --strict && pnpm check:migration-smoke-tests -- --strict` |
| complete-data-reconciled | Migration lead | blueprint:Criteres de Reussite | toutes les donnees migrables sont reconciliees | blocked | 1 | `toutes les donnees migrables sont reconciliees` | `tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json` |
| complete-gate-evidence | Migration lead | blueprint:Criteres de Reussite | chaque gate a une preuve attachee | blocked | 1 | `chaque gate a une preuve attachee` | `tools/migration/gate-evidence.mjs --strict` |
| complete-legacy-jobs-stopped | Migration lead | runbook:Validation finale | tous les jobs legacy sont arretes ou supprimes | blocked | 1 | `tous les jobs legacy sont arretes ou supprimes` | `tools/migration/job-map.mjs --strict && pnpm check:migration-decommission -- --strict` |
| complete-legacy-secrets-revoked | Migration lead | runbook:Validation finale | tous les secrets legacy inutiles sont revoques | blocked | 1 | `tous les secrets legacy inutiles sont revoques` | `tools/migration/secret-map.mjs --strict && pnpm check:migration-decommission -- --strict` |
| complete-live-cutover-evidence | Migration lead | runbook:Statut Operationnel | les preuves live de repetition, rollback, reconciliation, cutover et decommission sont attachees et acceptees | blocked | 1 | `les preuves live de repetition, rollback, reconciliation, cutover et decommission sont attachees et acceptees` | `tools/migration/live-evidence-instances.mjs --strict` |
| complete-no-runtime-legacy | Migration lead | blueprint:Criteres de Reussite | aucun composant runtime legacy n'est requis | blocked | 1 | `aucun composant runtime legacy n'est requis` | `pnpm check:migration-decommission -- --strict` |
| complete-no-v2-debt | Migration lead | runbook:Validation finale | aucun backlog V2 ne contient une dette necessaire au bon fonctionnement V1 | blocked | 1 | `aucun backlog V2 ne contient une dette necessaire au bon fonctionnement V1` | `pnpm check:migration-v2-debt -- --strict && tools/migration/completion-audit.mjs --strict` |
| complete-phase-acceptance | Migration lead | blueprint:Phases de Reconstruction | chaque phase de reconstruction a owner, preuve et decision go | blocked | 1 | `chaque phase de reconstruction a owner, preuve et decision go` | `tools/migration/phase-ledger.mjs --strict` |
| complete-post-audit-approved | Migration lead | runbook:Validation finale | post-migration-audit.md approuve par Migration lead, Security lead et owners produit | blocked | 1 | `post-migration-audit.md approuve par Migration lead, Security lead et owners produit` | `pnpm check:migration-post-migration-audit -- --strict` |
| complete-risk-closure | Migration lead | blueprint:Criteres de Reussite | chaque risque bloquant a ete ferme, accepte par owner, ou retire du scope | blocked | 1 | `chaque risque bloquant a ete ferme, accepte par owner, ou retire du scope` | `tools/migration/risk-register.mjs --strict` |
| complete-runbook-explicit | Migration lead | blueprint:Criteres de Reussite | le runbook de cutover peut etre execute sans decision implicite | blocked | 1 | `le runbook de cutover peut etre execute sans decision implicite` | `pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json` |
| resolve-gate_decisions | Migration lead | docs/migration/gate-evidence.generated.json | resolve and sign gate_decisions decisions | blocked | 5 | `gate_decisions unresolved item 1/5`<br>`gate_decisions unresolved item 2/5`<br>`gate_decisions unresolved item 3/5`<br>`gate_decisions unresolved item 4/5`<br>`gate_decisions unresolved item 5/5` | `tools/migration/gate-evidence.mjs --strict` |
| resolve-gates | Migration lead | docs/migration/gate-evidence.generated.json | resolve and sign gates decisions | blocked | 5 | `gates unresolved item 1/5`<br>`gates unresolved item 2/5`<br>`gates unresolved item 3/5`<br>`gates unresolved item 4/5`<br>`gates unresolved item 5/5` | `tools/migration/gate-evidence.mjs --strict` |
| resolve-live_evidence | Migration lead | docs/migration/live-evidence-instances.generated.json | resolve and sign live_evidence decisions | blocked | 6 | `live_evidence unresolved item 1/6`<br>`live_evidence unresolved item 2/6`<br>`live_evidence unresolved item 3/6`<br>`live_evidence unresolved item 4/6`<br>`live_evidence unresolved item 5/6`<br>`live_evidence unresolved item 6/6` | `tools/migration/live-evidence-instances.mjs --strict` |
| resolve-phases | Migration lead | docs/migration/phase-ledger.generated.json | resolve and sign phases decisions | blocked | 3 | `phases unresolved item 1/3`<br>`phases unresolved item 2/3`<br>`phases unresolved item 3/3` | `tools/migration/phase-ledger.mjs --strict` |
| resolve-reconciliation_template | Data lead | docs/migration/reconciliation.template.json | resolve and sign reconciliation_template decisions | blocked | 1 | `reconciliation_template unresolved item 1/1` | `tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json` |
| resolve-risks | Migration lead | docs/migration/risk-register.generated.json | resolve and sign risks decisions | blocked | 1 | `risks unresolved item 1/1` | `tools/migration/risk-register.mjs --strict` |

## Live Evidence Preparation

### attach-live-evidence-final-reconciliation

```bash
node tools/migration/live-evidence-prepare.mjs --id final-reconciliation-reconciliation-production-1 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/final-reconciliation-reconciliation-production-1 --packet-requirement final-reconciliation --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/final-reconciliation-reconciliation-production-1.json
```

### attach-live-evidence-g4-frontend-signoff

```bash
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-frontend-signoff-local-1 --type frontend_signoff --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-frontend-signoff-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-frontend-experience' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-frontend-signoff-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-frontend-signoff-local-1.json
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-web-check-local-1 --type web_check --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-web-check-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:web' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-web-check-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-web-check-local-1.json
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-smoke-test-local-1 --type smoke_test --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-smoke-test-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-smoke-tests -- --strict' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-smoke-test-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-smoke-test-local-1.json
```

### attach-live-evidence-g5-infra-signoff

```bash
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-infra-deploy-staging-1 --type infra_deploy --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-infra-deploy-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-infra-deploy' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-infra-deploy-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-infra-deploy-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-infra-restore-staging-1 --type infra_restore --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-infra-restore-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-backup-restore -- --strict' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-infra-restore-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-infra-restore-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-rollback-staging-1 --type rollback --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-rollback-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rollback-report -- --strict' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-rollback-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-rollback-staging-1.json
```

### attach-live-evidence-p11-rehearsals

```bash
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-local-1 --type rehearsal --env local --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-local-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-local-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-staging-2 --type rehearsal --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-staging-2.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-staging-2 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-staging-2.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-production-3 --type rehearsal --env production --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-production-3.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-production-3 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-production-3.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reconciliation-staging-1 --type reconciliation --env staging --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reconciliation-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reconciliation-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reconciliation-production-2 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reconciliation-production-2 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reconciliation-production-2.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rollback-staging-1 --type rollback --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rollback-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rollback-report -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rollback-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rollback-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reject-review-staging-1 --type reject_review --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-reject-review-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rejects -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reject-review-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reject-review-staging-1.json
```

### attach-live-evidence-p12-cutover

```bash
node tools/migration/live-evidence-prepare.mjs --id p12-cutover-cutover-production-1 --type cutover --env production --owner '<owner>' --source-artifact artifact://migration/p12-cutover-cutover-production-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p12-cutover-cutover-production-1 --packet-requirement p12-cutover --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p12-cutover-cutover-production-1.json
node tools/migration/live-evidence-prepare.mjs --id p12-cutover-reconciliation-production-1 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p12-cutover-reconciliation-production-1 --packet-requirement p12-cutover --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p12-cutover-reconciliation-production-1.json
```

### attach-live-evidence-p13-decommission

```bash
node tools/migration/live-evidence-prepare.mjs --id p13-decommission-decommission-production-1 --type decommission --env production --owner '<owner>' --source-artifact artifact://migration/p13-decommission-decommission-production-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-postcutover' --result passed --decision go --immutable-reference artifact://migration/p13-decommission-decommission-production-1 --packet-requirement p13-decommission --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p13-decommission-decommission-production-1.json
```


## Regeneration

```bash
pnpm check:migration-execution-backlog
tools/migration/execution-backlog.mjs --write
```
