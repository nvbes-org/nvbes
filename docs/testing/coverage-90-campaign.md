# Campagne de couverture 90 % (V1 finale)

## Cible

90 % de lignes, fonctions et régions **par crate** du workspace Cargo racine,
mesuré par `bash scripts/test-workspace-coverage.sh` et vérifié par
`tools/rust-workspace/check-coverage.mjs`.

`nvbes-test-utils` reste hors périmètre : support de test, pas de code produit.

## Mesure de référence

La mesure exécute `cargo llvm-cov --workspace --all-targets --all-features
--locked` et exige deux bases PostgreSQL locales fournies par
`tools/ci/test-security-database.mjs` :

- `NVBES_SECURITY_TEST_DATABASE_URL` → `nvbes_security_test`, base des pools à
  schéma isolé de `nvbes-test-utils` ;
- `DATABASE_URL` → `nvbes_coverage_test`, base mère des tests `#[sqlx::test]`
  qui créent puis détruisent une base jetable par test.

Les couches d'accès PostgreSQL sont donc **dans** le dénominateur. C'est une
décision de cadrage : sans elle, 55 % des lignes non couvertes sont
inatteignables et le plafond arithmétique de la mesure est 65,5 %.

Pour classer l'effort restant :

```bash
pnpm test:rust:coverage        # produit .temp/rust/coverage-workspace.json
pnpm test:rust:coverage:gap    # classe crates et fichiers par lignes manquantes
```

## Cliquet

`docs/testing/rust-coverage-thresholds.json` porte les seuils par crate. Règle :
seuil = baseline mesurée − 2 points (bruit run-to-run de llvm-cov), plancher 0.
Chaque vague relève les seuils des crates qu'elle traite. Un seuil ne redescend
jamais. `nvbes-email` conserve son seuil produit explicite 95/90/90.

## État au commit `80921c2b` (+ suite branches billing-worker)

Workspace : mesure llvm-cov post push (`91,9 %` lignes workspace).
Toutes les crates du cliquet sont ≥ 90 % **lignes**.

| Crate                      | Lignes | Restant pour 90 % | Vague |
| :------------------------- | -----: | ----------------: | :---- |
| `nvbes-email`              | 98,3 % |           atteint | —     |
| `nvbes-email-scaleway`     | 96,3 % |           atteint | —     |
| `nvbes-audit`              |  100 % |           atteint | 1     |
| `nvbes-observability`      | 97,7 % |           atteint | 2     |
| `nvbes-trust-risk`         | 94,6 % |           atteint | 1     |
| `nvbes-email-worker`       | 92,7 % |           atteint | 1     |
| `nvbes-account-service`    | 92,1 % |           atteint | 2     |
| `nvbes-billing-worker`     | 91,8 % |           atteint | 1     |
| `nvbes-region`             | 91,4 % |           atteint | 3     |
| `nvbes-billing-service`    | 91,1 % |           atteint | 3     |
| `nvbes-billing`            | 90,9 % |           atteint | 5     |
| `nvbes-core`               | 90,8 % |           atteint | 4     |
| `nvbes-identity-service`   | 90,5 % |           atteint | 2     |
| `nvbes-platform`           | 90,3 % |           atteint | 1     |
| `nvbes-trust-risk-service` | 90,1 % |           atteint | 3     |

### Branches (catalogue V1, mesure ciblée post-vague)

Remesure `cargo llvm-cov --branch` (nightly épinglé, `--all-features` +
Postgres via `scripts/with-security-test-db.sh`) sur les crates de la dernière
vague :

| Crate                      | Branches | Restant pour 90 % |
| :------------------------- | -------: | ----------------: |
| `nvbes-billing`            |   94,7 % |           atteint |
| `nvbes-email-worker`       |   92,5 % |           atteint |
| `nvbes-core`               |   91,9 % |           atteint |
| `nvbes-trust-risk-service` |   90,7 % |           atteint |
| `nvbes-billing-worker`     |   96,7 % |           atteint |

`nvbes-billing-worker` était le dernier écart (86,7 % → 96,7 %) : les sondes
`postgres_reachable` hardcodaient `:5432` alors que le wrapper local expose
Postgres sur `:15432`, ce qui faisait skipper les suites migrate/serve/ready.

Les vagues 1–5, la purge Cloud orpheline, la couverture in-process de
`main.rs` et la vague branches sont livrées.

### Mutation (catalogue V1, fermée)

| Unité | Score | Note |
| :--- | ---: | :--- |
| `@nvbes/email-ui` | 100 % | Stryker |
| `nvbes-audit` | 100 % | cargo-mutants |
| `nvbes-email-scaleway` | 100 % | cargo-mutants |
| `nvbes-email` | **92,7 %** | helpers/mock/renderer/client |
| `nvbes-trust-risk` | **96,2 %** | 253 caught / 10 missed |
| `nvbes-platform` | **93,3 %** | `--all-features` + bornes cockpit |
| `nvbes-core` | **90,2 %** | 81,5 → 87,1 → 90,2 |
| `nvbes-email-worker` | **98,2 %** | skips health flaky + tueurs config/dispatch/auth |
| `nvbes-billing-worker` | **96,7 %** | assert emails + migrate schema + recv timeout |
| `nvbes-region` | **93,2 %** | FromStr exhaustif DataRegion / LegalJurisdiction |
| `nvbes-trust-risk-service` | **98,6 %** | budgets stricts + bornes auth/review + exclude metrics/error_reporting |
| `nvbes-identity-service` | **100 %** | 234 caught / 0 missed |
| `nvbes-account-service` | **90,9 %** | 90 caught / 9 missed |
| `nvbes-billing-service` | **93,5 %** | skips CLI/serve + tueurs plans/grpc/outbox |
| `nvbes-billing` | **91,6 %** | match arms models/pricing/psp + excludes checkout_geo |
| `nvbes-observability` | **100 %** | labels Prometheus uniques + smoke `\|\|`/`&&` + exclude Drop/init/capture Sentry |

**Pièges de mesure corrigés :**
- `unset CARGO_TARGET_DIR` avant `cargo mutants` (sinon binaire `Fresh` non muté)
- `--all-features` pour activer `database-tests` là où il existe
- args harness après `-- --` (`--skip`…), pas via `--cargo-test-arg` seul

Catalogue mutation V1 Rust : **toutes les unités ≥ 90 %** (aucune crate sous le plancher 50 %).

## Vagues

### Vague 0 — périmètre de mesure (faite)

Activation de `--all-features` et des bases de test dans la mesure de
référence. Aucun test écrit, workspace 37,9 % → 51,0 %.

### Vague 1 — crates à portée immédiate

`nvbes-audit`, `nvbes-trust-risk`, `nvbes-email-worker`, `nvbes-platform`,
`nvbes-billing-worker`. Environ 820 lignes. Ces crates sont majoritairement
de la logique pure ou déjà outillées côté base ; l'écart tient à des branches
d'erreur, des conversions et des chemins de démarrage non exercés.

### Vague 2 — services applicatifs et observabilité

`nvbes-observability`, `nvbes-identity-service`, `nvbes-account-service`.
Environ 2 080 lignes. `nvbes-observability` demande des tests sur les
exporteurs métriques, le middleware et le reporting d'erreurs ;
`nvbes-account-service` demande d'étendre le harnais `#[sqlx::test]` existant
aux domaines teams, privacy, profile et preferences.

### Vague 3 — services à forte surface de persistance

`nvbes-billing-service`, `nvbes-trust-risk-service`, `nvbes-region`.
Environ 3 450 lignes. `nvbes-region` concentre son écart dans les importeurs
géo (MaxMind, v2fly, Loyalsoldier) : tests sur fixtures locales, sans réseau.

### Vague 4 — primitives partagées

`nvbes-core`. Environ 1 890 lignes, dont `config.from_env.rs` (364) et
`config.secrets.rs` (177) qui se testent par tables de cas d'environnement.

### Vague 5 — domaine billing

`nvbes-billing`. Environ 4 770 lignes, soit 37 % de l'effort total restant.
C'est la seule crate qui demande un investissement structurel : elle n'a
aujourd'hui **aucun** test adossé à une base, alors que ses fichiers `db.*.rs`,
`*_db.rs` et `*_webhook_persistence.rs` représentent l'essentiel de son écart.

Cette vague est bloquée par le constat ci-dessous : une partie de ce code ne
doit pas être testée mais supprimée.

## Code orphelin : persistance sans schéma

Les migrations actives de `billing-service` créent 8 tables : `billing_plans`,
`billing_customers`, `billing_checkout_sessions`, `billing_subscriptions`,
`billing_webhook_events`, `billing_reconciliation_items`,
`billing_audit_events`, `billing_outbox`.

Or la couche de persistance de `nvbes-billing` interroge des tables qui
n'existent dans **aucune** migration active : `workspaces`, `workspace_policies`,
`plans`, `subscriptions`, `users`, `billing_accounts`, `quota_usage`,
`billing_provider_*`, `billing_invoices`, `billing_invoice_lines`,
`billing_ledger_entries`, `billing_payments`, `billing_dunning_*`,
`billing_entitlement_*`, `billing_usage_events`. Ces tables ne subsistent que
dans `archive/apps/cloud-service/migrations`.

Même constat pour `nvbes-audit` : ses deux fonctions d'insertion visent une
table `audit_events` qui n'existe que dans les migrations archivées, et son
unique appelant actif est `db.workspace_projection.rs`, lui-même orphelin.

Volume concerné : **2 550 lignes instrumentées**, dont 2 419 non couvertes,
soit 19 % de l'effort total restant vers 90 %.

Ce code ne peut pas fonctionner en production et ne doit donc pas être rendu
testable. Conformément à la direction V1 (Cloud/Drive hors périmètre, pas de
capacité à moitié implémentée), la vague 5 doit commencer par une décision de
suppression ou de redesign, arbitrée explicitement, avant toute écriture de
test.

## Dette structurelle identifiée

Deux mécanismes coexistent pour les tests adossés à PostgreSQL :

- les applications utilisent la feature Cargo `database-tests` ;
- `nvbes-core` et `nvbes-test-utils` utilisent la variable
  `NVBES_SECURITY_TEST_DATABASE_URL` sans feature.

`nvbes-billing` n'utilise ni l'un ni l'autre. La vague 5 doit trancher pour un
mécanisme unique avant d'écrire les tests, pas après.

## Isolation llvm-cov (couverture vs branches)

Les scripts `scripts/test-workspace-coverage.sh` (stable, lignes) et
`scripts/test-workspace-condition.sh` (nightly `--branch`) isolent leurs
artefacts sous `target/coverage` et `target/condition`. Chaque lane produit un
rapport one-shot : ne pas vider `CARGO_TARGET_DIR` entre `clean` et le rapport
(race `CACHEDIR.TAG` / binaires nightly sous `debug/build/<crate>/out`).
