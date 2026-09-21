# SonarCloud Strict Quality Gate & Code Quality Standard

Ce document formalise la configuration, les règles d'exclusion et les critères
d'exigence maximale du **Quality Gate SonarCloud** pour le projet `nvbes`.

---

## 1. Principes directeurs

Le projet `nvbes` applique une politique de **zéro dette tolérée** sur le socle V1 actif.
La configuration SonarCloud est calibrée pour imposer le **niveau maximal de rigueur**
sur chaque Pull Request et commit poussé sur `main`.

### Propriétés fondamentales :

- **Clean as You Code** : Tout nouveau code doit satisfaire immédiatement aux plus hauts standards.
- **Blocage systématique en CI** : `sonar.qualitygate.wait=true` force le scanner à interroger l'API SonarCloud et à échouer immédiatement la CI si un seul critère n'est pas rempli.
- **Couverture exhaustive** : Rust (Axum/SQLx), TypeScript (React/Vite), Terraform/IaC, Shell et Dockerfiles.
- **Sécurité intégrée (SAST & CVE)** : Ingestion directe des rapports SARIF (OSV-Scanner, Trivy, Gitleaks) en plus des analyseurs natifs SonarCloud.

---

## 2. Périmètre & Exclusion stricte des archives

### Exclusion du dossier `archive/`

Conformément à la direction produit V1 (`AGENTS.md`), les applications archivées
(`archive/account-web`, `archive/developer-portal`, etc.) représentent des prototypes
ou versions antérieures hors du runtime actif.

Dans [`sonar-project.properties`](../../sonar-project.properties) :

- `sonar.exclusions`: `archive/**, ...`
- `sonar.test.exclusions`: `archive/**`
- `sonar.coverage.exclusions`: `archive/**, ...`
- `sonar.cpd.exclusions`: `archive/**, ...`

> [!IMPORTANT]
> L'exclusion d'`archive/**` est stricte : aucun fichier archivé ne peut impacter
> le score de maintenabilité, la duplication ou la couverture de la V1 active.

### Autres exclusions techniques :

- Artefacts de build et caches : `**/target/**`, `**/.nx/**`, `**/.temp/**`, `**/dist/**`, `**/node_modules/**`
- Code généré automatiquement : `libs/ts/identity-sdk-core/src/generated/**`, `**/generated/**`, `**/*.generated.*`
- Fichiers minifiés et bundles : `**/*.min.js`, `**/*.bundle.js`, `**/*.map`
- Logs et données locales : `infrastructure/local/observability/app-logs/**`

### Exclusions multicriteria documentées (`sonar.issue.ignore.multicriteria`)

Quelques règles d'analyse **sécurité/js-security** émettent des findings sur du
tooling CI/opérateur mono-répo dont le modèle de menace ne s'applique pas
(entrées CLI/environnement qui _sont_ l'interface du script, binaires épinglés,
exécution uniquement dans les jobs CI ou terminaux opérateur de ce dépôt, jamais
dans un service déployé). Ces exclusions sont **documentées avec leur
justification** dans [`sonar-project.properties`](../../sonar-project.properties)
(blocs `e1..e10`) plutôt que remontées comme issues :

- `e1/e2` — `javascript:S4036` (`tools/**`, `scripts/**`) : résolution binaire via `PATH` (portabilité macOS/CI).
- `e3/e4` — `jssecurity:S8707` (`tools/**`, `scripts/**`) : chemins fournis par CLI/env = interface du tooling, confinement au workspace racine.
- `e5` — `jssecurity:S8476` (`tools/ci/runner-fallback.mjs`) : réutilisation de champs GitHub API derrière une origine épinglée.
- `e6/e7` — `tssecurity:S8707` (`tools/**`, `scripts/**`) : équivalent TypeScript de `e3/e4` (ex. `check-bundle-size.ts`, `vite-sri.ts`).
- `e8/e9` — `jssecurity:S8705` (`v1-receipt-assembly.mjs`, `upload-grafana-sourcemaps.mjs`) : 'gh api' / 'faro-cli' avec validateurs de forme d'argument.
- `e10` — `jssecurity:S6350` (`scripts/lib/stripe-sandbox.mjs`) : sandbox opérateur local, whitelist de binaires, métadonnées validées (`^price_[A-Za-z0-9]+$`).

> [!IMPORTANT]
> Toute nouvelle exclusion multicriteria exige une justification écrite du
> modèle de menace dans `sonar-project.properties` ; une exclusion sans motif ne
> sera pas acceptée.

---

## 3. Critères du Quality Gate Strict ("nvbes Strict Quality Gate")

Dans l'interface SonarCloud (**Quality Gates > Create / Edit**), le Quality Gate
associé au projet `nvbes-org_nvbes` est configuré avec les seuils suivants sur le **Nouveau Code** (_New Code_) :

| Domaine            | Métrique                        | Opérateur | Seuil strict | Description                                          |
| :----------------- | :------------------------------ | :-------: | :----------: | :--------------------------------------------------- |
| **Fiabilité**      | New Bugs                        |    $=$    |    **0**     | Aucun nouveau bug accepté (Rating A obligatoire)     |
| **Sécurité**       | New Vulnerabilities             |    $=$    |    **0**     | Aucune nouvelle vulnérabilité (Rating A obligatoire) |
| **Sécurité**       | New Security Hotspots Reviewed  |    $=$    |  **100 %**   | 100 % des hotspots doivent être audités et justifiés |
| **Maintenabilité** | New Code Smells                 |    $=$    |    **0**     | Aucun code smell toléré sur le nouveau code          |
| **Maintenabilité** | Technical Debt Ratio (New Code) |   $\le$   |  **5.0 %**   | Dette technique strictement bornée (Rating A)        |
| **Couverture**     | Coverage on New Code            |   $\ge$   |  **80.0 %**  | Couverture minimale sur tout code ajouté ou modifié  |
| **Duplication**    | Duplicated Lines on New Code    |   $\le$   |  **3.0 %**   | Tolérance minimale sur les répétitions de blocs      |
| **Gravité**        | New Blocker / Critical Issues   |    $=$    |    **0**     | Blocage absolu sur toute anomalie majeure            |

---

## 4. Multi-langages, Linting & Rapports ingérés

SonarCloud agrège les signaux de linting, couverture et sécurité issus de l'écosystème du monorepo :

### 4.1. Rust (Backend & Libs)

- **Couverture de code (LCOV)** :
  - Générée par `cargo llvm-cov` via [`scripts/test-workspace-coverage.sh`](../../scripts/test-workspace-coverage.sh).
  - Emplacement : `.temp/rust/lcov.info`.
  - Configuré via `sonar.rust.lcov.reportPaths=.temp/rust/lcov.info`.
- **Clippy (Linter & Standard Quality)** :
  - Généré au format JSON via `cargo clippy --workspace --all-targets --locked --message-format=json`.
  - Emplacement : `.temp/rust/clippy-report.json`.
  - Configuré via `sonar.rust.clippyReport.reportPaths=.temp/rust/clippy-report.json`.

### 4.2. TypeScript & Frontend

- **Couverture de code** :
  - Emplacement : `coverage/lcov.info`, `libs/ts/*/coverage/lcov.info`.
  - Configuré via `sonar.javascript.lcov.reportPaths`.
- **Lint & Format** :
  - Vérifié en amont par Biome (`vp lint` / `vp fmt`) et `tsc`.

### 4.3. CVEs & Sécurité des dépendances (SARIF)

- **Rapports ingérés** :
  - `osv-results.sarif` : Vulnérabilités de dépendances (OSV-Scanner sur `Cargo.lock` et `pnpm-lock.yaml`).
  - `trivy-results.sarif` : Vulnérabilités de conteneurs, système de fichiers et IaC.
  - `gitleaks-results.sarif` : Détection de secrets dans l'historique git.
  - Configuré via `sonar.sarifReportPaths=osv-results.sarif,trivy-results.sarif,gitleaks-results.sarif`.

---

## 5. Intégration CI/CD GitHub Actions

Le workflow [`.github/workflows/sonarcloud.yml`](../../.github/workflows/sonarcloud.yml) respecte l'ensemble
des exigences du registre de sécurité du repo :

1. **Gate de sécurité préalable** : Exécution immédiate de `node tools/security/check-ci-cd-security.mjs` avant toute installation.
2. **Action officielle épinglée par commit SHA complet** :
   `SonarSource/sonarqube-scan-action@299e4b793aaa83bf2aba7c9c14bedbb485688ec4` (v7.1.0, support Node 24).
3. **Moindre privilège** :
   `permissions: actions: read, contents: read, pull-requests: read`.
4. **Secret sécurisé** : `SONAR_TOKEN` enregistré et restreint à ce workflow spécifique dans `docs/security/ci-cd-security-controls.json` et `tools/security/ci-secret-policy.mjs`.

---

## 6. Commandes utiles locales

```bash
# Préparer les rapports Clippy et couverture pour SonarCloud
pnpm sonar:prepare

# Vérifier les contrôles CI/CD associés aux workflows
pnpm check:ci-cd-security
pnpm check:workflows
```
