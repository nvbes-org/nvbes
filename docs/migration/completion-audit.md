# Migration Completion Audit

## Status

- objective: incomplete
- requirements: 24
- incomplete: 13
- readiness: no-go
- readiness_blocking_items: 23
- runtimes_go: 4/4
- domains_go: 8/8
- domain_dod_go: 64/64
- gates_go: 4/9
- phases_go: 10/13
- live_evidence_decision: no-go
- live_evidence_missing_requirements: 6
- live_evidence_missing_items: 24

## Rules

- `requirements` must match the generated requirement rows.
- `incomplete` must equal rows that are not `complete` or `control-active`.
- `objective: complete` requires zero incomplete rows, readiness `go`, live evidence `go`, and zero blockers.
- `objective: complete` requires all runtime, domain, DoD, gate and phase counters to be fully `go`.
- Every requirement must carry source, requirement text, evidence and a proof command.
- Generation provenance must identify write and strict completion commands.

## Requirements

| ID | Source | Requirement | Status | Evidence | Proof |
|---|---|---|---:|---|---|
| critical-journeys | blueprint:Criteres de Reussite | tous les parcours critiques ont une implementation nouvelle | blocked | `docs/migration/parity-matrix.md`<br>`docs/migration/smoke-test-manifest.md` | `pnpm check:migration-parity -- --strict && pnpm check:migration-smoke-tests -- --strict` |
| contracts-versioned | blueprint:Criteres de Reussite | tous les contrats publics et internes sont versionnes | control-active | `contracts/`<br>`tools/contracts/checks.mjs` | `pnpm check:contracts` |
| sdk-codegen | blueprint:Contrats | les contrats OpenAPI, Protobuf et events ont une couverture SDK TypeScript, Rust et Go | complete | `docs/migration/codegen.generated.json` | `pnpm check:codegen && tools/migration/codegen.mjs --strict` |
| data-reconciled | blueprint:Criteres de Reussite | toutes les donnees migrables sont reconciliees | blocked | `docs/migration/reconciliation.template.json`<br>`docs/migration/readiness-report.generated.json` | `tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json` |
| target-monorepo-structure | blueprint:Structure Monorepo Cible | la structure monorepo cible existe pour les frontieres apps, libs, contracts, deploy, docs et tools | complete | `docs/migration/target-structure.generated.json` | `tools/migration/target-structure.mjs --strict` |
| supply-chain-scans | blueprint:Fondation Technique | les scans secrets, licences, SBOM, dependances et containers sont actifs | complete | `docs/migration/supply-chain.generated.json`<br>`docs/migration/sbom.generated.json` | `pnpm check:supply-chain && tools/migration/supply-chain.mjs --strict` |
| phase-acceptance | blueprint:Phases de Reconstruction | chaque phase de reconstruction a owner, preuve et decision go | blocked | `docs/migration/phase-ledger.generated.json` | `tools/migration/phase-ledger.mjs --strict` |
| runtime-foundation | blueprint:Stack Cible | les fondations Rust, Go, TypeScript et Python existent et ont leurs checks CI | complete | `docs/migration/runtime-foundation.generated.json` | `tools/migration/runtime-foundation.mjs --strict` |
| domain-acceptance | blueprint:Frontieres de Domaine | chaque domaine de base a implementation, preuve migration et decision go | complete | `docs/migration/domain-ledger.generated.json` | `tools/migration/domain-ledger.mjs --strict` |
| domain-definition-of-done | blueprint:Definition de Done Domaine | chaque domaine de base satisfait tous les criteres de Definition of Done | complete | `docs/migration/domain-dod.generated.json` | `tools/migration/domain-dod.mjs --strict` |
| no-runtime-legacy | blueprint:Criteres de Reussite | aucun composant runtime legacy n'est requis | blocked | `docs/migration/decommission-manifest.md` | `pnpm check:migration-decommission -- --strict` |
| provider-boundaries | blueprint:Criteres de Reussite | aucun provider proprietaire n'est dans le core OSS | control-active | `scripts/check-oss-provider-boundaries.mjs` | `pnpm check:oss-boundaries` |
| forbidden-imports | blueprint:Criteres de Reussite | aucun import interdit OSS/Cloud/Internal ne passe la CI | control-active | `tools/boundary-checks/check-product-boundaries.mjs`<br>`scripts/check-nx-boundaries.mjs` | `pnpm check:product-boundaries && pnpm check:oss-boundaries` |
| backup-rollback | blueprint:Criteres de Reussite | les backups et rollbacks sont testes | blocked | `docs/migration/backup-restore-manifest.md`<br>`docs/migration/rollback-report.md` | `pnpm check:migration-backup-restore -- --strict && pnpm check:migration-rollback-report -- --strict` |
| oss-export-private-docs | blueprint:Criteres de Reussite | le repo public OSS ne contient aucun document prive | control-active | `tools/oss-export/checks.mjs` | `pnpm check:oss-boundaries` |
| runbook-explicit | blueprint:Criteres de Reussite | le runbook de cutover peut etre execute sans decision implicite | blocked | `docs/migration/nvbes-big-bang-migration-runbook.md`<br>`docs/migration/cutover-evidence-packet.generated.json`<br>`docs/migration/live-evidence.schema.json`<br>`docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/readiness-report.generated.json` | `pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json` |
| live-cutover-evidence | runbook:Statut Operationnel | les preuves live de repetition, rollback, reconciliation, cutover et decommission sont attachees et acceptees | blocked | `docs/migration/live-evidence-instances.generated.json`<br>`docs/migration/live-evidence-instances.md` | `tools/migration/live-evidence-instances.mjs --strict` |
| gate-evidence | blueprint:Criteres de Reussite | chaque gate a une preuve attachee | blocked | `docs/migration/gate-evidence.generated.json` | `tools/migration/gate-evidence.mjs --strict` |
| risk-closure | blueprint:Criteres de Reussite | chaque risque bloquant a ete ferme, accepte par owner, ou retire du scope | blocked | `docs/migration/risk-register.generated.json` | `tools/migration/risk-register.mjs --strict` |
| post-audit-approved | runbook:Validation finale | post-migration-audit.md approuve par Migration lead, Security lead et owners produit | blocked | `docs/migration/post-migration-audit.md` | `pnpm check:migration-post-migration-audit -- --strict` |
| legacy-secrets-revoked | runbook:Validation finale | tous les secrets legacy inutiles sont revoques | blocked | `docs/migration/secret-map.generated.json`<br>`docs/migration/decommission-manifest.md` | `tools/migration/secret-map.mjs --strict && pnpm check:migration-decommission -- --strict` |
| legacy-jobs-stopped | runbook:Validation finale | tous les jobs legacy sont arretes ou supprimes | blocked | `docs/migration/job-map.generated.json`<br>`docs/migration/decommission-manifest.md` | `tools/migration/job-map.mjs --strict && pnpm check:migration-decommission -- --strict` |
| docs-updated | runbook:Validation finale | tous les runbooks publics et prives sont a jour | control-active | `docs/migration/`<br>`docs/blueprint/` | `pnpm check:migration-artifacts && pnpm check:migration-runbook && pnpm check:migration-blueprint` |
| no-v2-debt | runbook:Validation finale | aucun backlog V2 ne contient une dette necessaire au bon fonctionnement V1 | blocked | `docs/migration/v2-debt-register.md` | `pnpm check:migration-v2-debt -- --strict && tools/migration/completion-audit.mjs --strict` |

## Regeneration

```bash
pnpm check:migration-completion-audit
tools/migration/completion-audit.mjs --write
```
