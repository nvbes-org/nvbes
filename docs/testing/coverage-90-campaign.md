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

## État au commit `7a342eb7`

Workspace : mesure llvm-cov (`pnpm test:rust:coverage`) sur la vague courante.
Crates ≥ 90 % lignes : `nvbes-audit`, `nvbes-email`, `nvbes-email-scaleway`,
`nvbes-trust-risk`, `nvbes-platform`, `nvbes-region`, `nvbes-trust-risk-service`,
`nvbes-account-service`, `nvbes-observability`.

| Crate                      | Lignes | Restant pour 90 % | Vague |
| :------------------------- | -----: | ----------------: | :---- |
| `nvbes-email`              | 98,3 % |           atteint | —     |
| `nvbes-email-scaleway`     | 96,3 % |           atteint | —     |
| `nvbes-audit`              |  100 % |           atteint | 1     |
| `nvbes-observability`      | 97,7 % |           atteint | 2     |
| `nvbes-trust-risk`         | 94,6 % |           atteint | 1     |
| `nvbes-account-service`    | 92,1 % |           atteint | 2     |
| `nvbes-region`             | 91,4 % |           atteint | 3     |
| `nvbes-platform`           | 90,3 % |           atteint | 1     |
| `nvbes-trust-risk-service` | 90,1 % |           atteint | 3     |
| `nvbes-billing`            | 89,7 % |                12 | 5     |
| `nvbes-email-worker`       | 87,6 % |                71 | 1     |
| `nvbes-core`               | 86,9 % |               131 | 4     |
| `nvbes-identity-service`   | 84,5 % |               127 | 2     |
| `nvbes-billing-service`    | 79,2 % |               210 | 3     |
| `nvbes-billing-worker`     | 78,5 % |                80 | 1     |

Les vagues 1–5 et la purge Cloud orpheline sont livrées structurellement ;
l'effort restant porte sur les lignes/branches/mutations catalogue V1
(pas seulement le cliquet).

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
