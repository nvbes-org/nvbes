# Live Evidence Instances

## Status

- instance_count: 0
- requirements: 6
- missing_requirements: 6
- missing_evidence_items: 24
- decision: no-go

## Requirements

| Requirement | Scope | Expected Evidence | Missing Evidence | Instances | Ready |
|---|---|---|---|---:|---:|
| g4-frontend-signoff | G4 Frontends | frontend_signoff x1, web_check x1, smoke_test x1 | frontend_signoff:0/1, web_check:0/1, smoke_test:0/1 | 0 | false |
| g5-infra-signoff | G5 Infra | infra_deploy x1 (staging), infra_restore x1 (staging), rollback x1 (staging) | infra_deploy:0/1, infra_deploy@staging, infra_restore:0/1, infra_restore@staging, rollback:0/1, rollback@staging | 0 | false |
| p11-rehearsals | P11/G6 Repetitions migration | rehearsal x3 (local,staging,production), reconciliation x2, rollback x1, reject_review x1 | rehearsal:0/3, rehearsal@local, rehearsal@staging, rehearsal@production, reconciliation:0/2, rollback:0/1, reject_review:0/1 | 0 | false |
| p12-cutover | P12/G7 Big Bang cutover | cutover x1 (production), reconciliation x1 (production) | cutover:0/1, cutover@production, reconciliation:0/1, reconciliation@production | 0 | false |
| p13-decommission | P13/G8 Decommission | decommission x1 (production) | decommission:0/1, decommission@production | 0 | false |
| final-reconciliation | Production reconciliation | reconciliation x1 (production) | reconciliation:0/1, reconciliation@production | 0 | false |

## Attached Instances

No live evidence instances attached.


## Validation Rules

- accepted evidence must use `decision: go` with `result: passed` or `accepted`;
- accepted evidence must include notes with owner justification;
- `captured_at` must be parseable and cannot be in the future;
- go evidence must use `ci://`, `artifact://`, `s3://`, `gs://` or `oci://` in `immutable_reference`; generic web URLs and local mutable paths are rejected;
- go evidence external `source_artifact` references must use `ci://`, `artifact://`, `s3://`, `gs://` or `oci://`; generic web URLs are rejected;
- accepted go evidence must use a unique `immutable_reference`; one artifact cannot satisfy multiple evidence instances;
- accepted go evidence must use a unique `source_artifact`; one source artifact cannot satisfy multiple evidence instances;
- accepted go evidence command must match one segment of the owning cutover packet `strict_command`; concrete run IDs may replace `<run>`;
- accepted local source artifacts must be referenced by the owning packet requirement unless they are the command `--report` output;
- accepted local source artifacts must stay under `docs/migration/` or the migration fixtures directory; arbitrary repository paths are rejected;
- `evidence_id` must use the approved lowercase slug format and match the JSON instance filename;
- when a go evidence command uses `--report`, `source_artifact` must be the same report path;
- when a go evidence command uses `--env`, it must match the instance `environment`; both `--env value` and `--env=value` are supported;
- go reconciliation evidence with a local JSON `source_artifact` must have a valid go or go-with-accepted-rejects reconciliation report;
- go evidence must include a checksum named `source_artifact`; unrelated checksum names are rejected;
- go evidence must include a checksum named `cutover_evidence_packet` matching `docs/migration/cutover-evidence-packet.generated.json`; stale packet evidence is rejected;
- generation provenance must identify write and strict check commands;
- requirement ids, scopes, strict commands and expected evidence must match the cutover evidence packet contract;
- go evidence cannot reference `.template.` artifacts or template commands;
- failed or blocking evidence must stay `decision: no-go`;
- each instance must match the evidence types and environments allowed for its packet requirement;
- checksum names must be unique and hexadecimal; local `source_artifact` files must match the checksum named `source_artifact`; unrelated checksums cannot satisfy source artifact proof;
- placeholder IDs, owners, artifact names, references and zero checksums are rejected.

## Instance Location

Copy validated live evidence JSON files into `docs/migration/live-evidence-instances/`. Start from `docs/migration/live-evidence.template.json` and keep immutable artifact references plus checksums.

## Preparation Commands

Use these commands after the real run artifact exists. Replace `<run>`, `<owner>`, `<source-artifact-sha256>` and the notes before attaching the generated JSON.

```bash
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-frontend-signoff-local-1 --type frontend_signoff --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-frontend-signoff-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-frontend-experience' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-frontend-signoff-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-frontend-signoff-local-1.json
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-web-check-local-1 --type web_check --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-web-check-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:web' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-web-check-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-web-check-local-1.json
node tools/migration/live-evidence-prepare.mjs --id g4-frontend-signoff-smoke-test-local-1 --type smoke_test --env local --owner '<owner>' --source-artifact artifact://migration/g4-frontend-signoff-smoke-test-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-smoke-tests -- --strict' --result passed --decision go --immutable-reference artifact://migration/g4-frontend-signoff-smoke-test-local-1 --packet-requirement g4-frontend-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g4-frontend-signoff-smoke-test-local-1.json
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-infra-deploy-staging-1 --type infra_deploy --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-infra-deploy-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-infra-deploy' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-infra-deploy-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-infra-deploy-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-infra-restore-staging-1 --type infra_restore --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-infra-restore-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-backup-restore -- --strict' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-infra-restore-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-infra-restore-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id g5-infra-signoff-rollback-staging-1 --type rollback --env staging --owner '<owner>' --source-artifact artifact://migration/g5-infra-signoff-rollback-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rollback-report -- --strict' --result passed --decision go --immutable-reference artifact://migration/g5-infra-signoff-rollback-staging-1 --packet-requirement g5-infra-signoff --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/g5-infra-signoff-rollback-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-local-1 --type rehearsal --env local --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-local-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-local-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-local-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-staging-2 --type rehearsal --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-staging-2.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-staging-2 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-staging-2.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rehearsal-production-3 --type rehearsal --env production --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rehearsal-production-3.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rehearsals -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rehearsal-production-3 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rehearsal-production-3.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reconciliation-staging-1 --type reconciliation --env staging --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reconciliation-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reconciliation-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reconciliation-production-2 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reconciliation-production-2 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reconciliation-production-2.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-rollback-staging-1 --type rollback --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-rollback-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rollback-report -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-rollback-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-rollback-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id p11-rehearsals-reject-review-staging-1 --type reject_review --env staging --owner '<owner>' --source-artifact artifact://migration/p11-rehearsals-reject-review-staging-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-rejects -- --strict' --result passed --decision go --immutable-reference artifact://migration/p11-rehearsals-reject-review-staging-1 --packet-requirement p11-rehearsals --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p11-rehearsals-reject-review-staging-1.json
node tools/migration/live-evidence-prepare.mjs --id p12-cutover-cutover-production-1 --type cutover --env production --owner '<owner>' --source-artifact artifact://migration/p12-cutover-cutover-production-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p12-cutover-cutover-production-1 --packet-requirement p12-cutover --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p12-cutover-cutover-production-1.json
node tools/migration/live-evidence-prepare.mjs --id p12-cutover-reconciliation-production-1 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/p12-cutover-reconciliation-production-1 --packet-requirement p12-cutover --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p12-cutover-reconciliation-production-1.json
node tools/migration/live-evidence-prepare.mjs --id p13-decommission-decommission-production-1 --type decommission --env production --owner '<owner>' --source-artifact artifact://migration/p13-decommission-decommission-production-1.source --source-checksum '<source-artifact-sha256>' --command 'pnpm check:migration-postcutover' --result passed --decision go --immutable-reference artifact://migration/p13-decommission-decommission-production-1 --packet-requirement p13-decommission --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/p13-decommission-decommission-production-1.json
node tools/migration/live-evidence-prepare.mjs --id final-reconciliation-reconciliation-production-1 --type reconciliation --env production --owner '<owner>' --source-artifact 'docs/migration/reconciliation.<run>.json' --command 'node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json' --result passed --decision go --immutable-reference artifact://migration/final-reconciliation-reconciliation-production-1 --packet-requirement final-reconciliation --notes '<owner justification and run context>' --out docs/migration/live-evidence-instances/final-reconciliation-reconciliation-production-1.json
```

## Verification

```bash
pnpm check:migration-live-evidence-instances
pnpm check:migration-live-evidence-instances -- --strict
```
