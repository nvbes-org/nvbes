# nvbes Project — Guide Canonique des Agents LLM

Ce fichier est la **source de vérité unique et consolidée** pour tous les assistants et agents LLM (Antigravity, Codex, Claude Code, Cursor, Copilot, Windsurf) intervenant sur le monorepo nvbes.

---

## 1. Direction V1 Active & Contrat FinOps

La [direction produit canonique](docs/product/nvbes-product-strategy.md) prévaut impérativement sur tous les anciens PRD, blueprints et plans historiques :

- **Mission Nvbes** : Nvbes est une entité destinée à offrir des outils et des logiciels aux développeurs, le tout en **open-source**, financé par son futur cloud.
- **V1 Ultra-Restreinte ("Small Building")** : La V1 sert exclusivement de service d'identité et de compte pour les développeurs utilisant les outils Nvbes, avec une configuration interne et manuelle (pas de portail d'enregistrement d'apps complexe en V1) :
  - **Une API monolithique** (`apps/api`) : Création de compte, Connexion et maintien de session le tout en **OAuth 2** (PKCE, RFC 6749), gestion des consentements (scopes) et profil utilisateur basique.
  - **Un site web dev-first** (aujourd'hui `apps/identity-web` et `apps/account-web`, unification prévue) : Inscription, Connexion en OAuth2, écran de consentement, page de profil (infos, sessions actives). Destiné uniquement aux développeurs : **aucun front-end marketing en V1**.
- **Deux Principes Majeurs** :
  1. **Rentabilité maximale & technologies les moins chères du marché** : Exploiter au maximum la gratuité et l'ultra-légèreté (Rust/Axum < 20Mo RAM, Cloudflare Pages gratuit pour le front, PostgreSQL managé ou low-cost).
  2. **Migration vers l'Open Source auto-géré** : Chaque brique et choix technologique doit être conçu pour permettre une migration fluide vers des solutions OSS auto-hébergées dans le futur (PostgreSQL natif, reverse proxy Nginx/Caddy, zéro lock-in propriétaire).
- **Hors périmètre actif V1 (Gelé / Mis en pause)** :
  - ❌ Pas de **Billing (Stripe)** pour l'instant (activé ultérieurement lors du lancement du cloud payant).
  - ❌ Pas de **Trust/Risk engine** ni de workers distribués : l'API reste strictement serverless (aucune tâche de fond) et un seul binaire `apps/worker` exécute les tâches, en Scaleway Container (`serve`) ou en Scaleway Job (`run <task>`).
  - ❌ Pas de **front-end marketing** ni de gestion complexe d'organisations/équipes B2B.
  - ❌ **Souveraineté et conformité avancée non exigées en V1** : préparées dans les abstractions, mais sans surcoût ni complexité bloquante pour démarrer.
- **Plafond FinOps impératif** : Le coût récurrent total de l'infrastructure ne doit jamais dépasser **30 EUR TTC par mois** (cible opérationnelle à 20 EUR). FinOps est une contrainte de conception et un gate de livraison non négociable.
- **Opérateur solo** : L'administration, le support et la maintenance restent manuels par défaut. Une automatisation n'est recevable que sur un besoin mesuré, sécurisé, réversible et compatible avec le budget.
- **Zéro dette technique livrée** : Aucune dette structurelle ou temporaire n'est acceptée dans le périmètre V1 livré. Si un design est bancal, le **redesign complet** est hautement encouragé. Après V1, toute dette doit avoir un propriétaire, un impact mesuré, une date de revue et une condition de résolution.
- **Production progressive** : Les déploiements sont mesurés, progressifs et immédiatement réversibles.

---

## 2. Stack & Arborescence Active (Modular Monolith)

### Stack Technique

- **Gestionnaire de paquets** : pnpm (workspaces) + Cargo (workspace Rust)
- **Monorepo tooling** : Nx (graphe de tâches, affectation, exécution ciblée)
- **Formatage & Lint** : Biome (`vp check`, `vp fmt`) + `cargo fmt` + `cargo clippy`
- **Hooks Git** : Lefthook
- **Backend** : Rust stable (2021), Axum, SQLx (mode offline `query!`), PostgreSQL
- **Frontend** : React 19, Vite, TanStack Router, Tailwind CSS, shadcn/ui (Radix), Lucide
- **Auth** : nvbes OAuth 2.0 / PKCE (RFC 6749, RFC 7636), Sessions, Consentement, JWT & DPoP
- **Hébergement & Infra** : Cloudflare Pages (Frontend gratuit), Runtimes ultra-frugaux (Axum < 20Mo RAM), PostgreSQL managé / low-cost
- **Billing** : Hors périmètre actif V1 (gelé jusqu'au lancement commercial du Cloud)

### Structure du Dépôt

```
nvbes/
├── apps/
│   ├── api/                # API serverless (Axum) : Identity + Account, aucune tâche de fond
│   ├── worker/             # Worker unique : `serve` (Container) ou `run <task>` (Job)
│   ├── identity-web/       # UI hébergée Identity (OAuth/MFA, récupération, React/Vite)
│   └── account-web/        # UI Account (profil, consentements, RGPD, React/Vite)
├── libs/
│   ├── rust/
│   │   ├── identity/            # Domaine Identity (OAuth 2.1, OIDC, WebAuthn, MFA, récupération)
│   │   ├── account/             # Domaine Account (profil, préférences, consentements légaux, RGPD)
│   │   ├── identity-sdk-backend/# Vérification JWT et ActivityVerifier pour serveurs de ressources
│   │   ├── dpop/                # Preuves DPoP (RFC 9449)
│   │   ├── email/               # Commandes, gabarits et livraison email (SMTP, Scaleway TEM)
│   │   ├── adapters/            # Adaptateurs externes (Scaleway TEM)
│   │   ├── core/                # Primitives partagées (http, sécurité, crypto, mfa)
│   │   ├── observability/       # Métriques Prometheus, tracing OpenTelemetry, Sentry
│   │   └── test-utils/          # Runner de tests par mots-clés
│   └── ts/
│       ├── http-client/        # Client fetch partagé (DTO + helpers DPoP)
│       ├── identity-sdk-core/  # Types OpenAPI générés
│       ├── identity-sdk-web/   # SDK navigateur Identity (OAuth/WebAuthn)
│       ├── account-sdk-core/   # Contrat OpenAPI Account
│       └── email-ui/           # Templates React Email actifs
├── infrastructure/         # Terraform/OpenTofu (api, worker, bootstrap)
└── archive/                # Services et crates gelés (billing, trust-risk, consent, ...)
```

*Note* : Les applications archivées (`archive/apps/`) ne sont pas des bases actives. Ne pas copier le code archivé par défaut.

---

## 3. Contrat Agent & Règles d'Opération Globales (100% LLM)

- **Workflow 100% LLM** : L'agent a l'autorité technique pour modifier tout fichier du dépôt pour accomplir sa mission, y compris les fichiers de configuration de gates (`lefthook.yml`, `tools/ci/**`, `tools/security/**`, `docs/testing/*thresholds*`, `deny.toml`, `.github/**`, `commitlint.config.cjs`, `AGENTS.md`). Il n'y a aucun gate de reprise humaine bloquant.
- **Commit Trailer obligatoire** : Tout commit LLM DOIT obligatoirement comporter le trailer `AI-Assisted: <agent-or-model>` (ex: `AI-Assisted: Antigravity`).
- **Conventional Commits** : Messages de commit strictement conventionnels (`feat:`, `fix:`, `chore:`, `refactor:`, etc.). Lignes de résumé et de corps strictement inférieures à **80 caractères**.
- **Respect du dirty worktree** : L'espace de travail peut comporter des modifications utilisateur en cours. Ne JAMAIS annuler, écraser ou reverter des modifications non liées à la tâche demandée.
- **Outil de recherche local** : Utiliser impérativement `rg` (ripgrep) pour le texte et `rg --files` pour les fichiers. N'utiliser `grep` ou `find` qu'en solution de repli si `rg` est inadapté.
- **Grounding certifié** : Avant d'inférer des noms de crates, des cibles Nx ou des routes OpenAPI réelles, consulter `.docgen/workspace-ir.json`.
- **Diagrammes Mermaid** : Tout participant ou acteur avec des espaces ou caractères spéciaux DOIT être quoter (`participant DB as "PostgreSQL (Billing DB)"`).
- **Seuils monotones** : Interdiction absolue d'abaisser un seuil de couverture, de mutation ou de condition dans `docs/testing/` ou d'exclure un crate (`pnpm check:thresholds-monotone`).
- **Unwrap Ratchet** : Zéro nouveau `.unwrap()` ou `.expect()` en code de production Rust (`pnpm check:unwrap-ratchet`).
- **SQLx Ratchet** : Zéro nouveau `sqlx::query(` runtime ; utiliser `sqlx::query!` avec métadonnées `.sqlx` hors-ligne synchronisées (`pnpm check:sqlx-ratchet`).
- **Assertions rigoureuses** : Préférer `matches!` ou `assert_eq!` aux assertions faibles `assert!(x.is_ok())` / `assert!(x.is_err())` (`pnpm check:weak-assertions`).

---

## 4. RTK (Rust Token Killer) — Règle Shell & Proxy CLI

Pour minimiser la consommation de tokens de contexte (économie de 60 à 90%), préfixer systématiquement les commandes shell par `rtk` :

```bash
rtk cargo test
rtk git status
rtk pnpm check
rtk docker ps
rtk rg "pattern"
```

### Commandes Méta RTK
- `rtk gain` : Affiche les gains de tokens et statistiques de compression.
- `rtk gain --history` : Historique des commandes avec gains associés.
- `rtk discover` : Détecte les opportunités d'optimisation manquées.
- `rtk proxy <cmd>` : Exécution brute sans filtrage (à utiliser uniquement pour le débogage).

---

## 5. Routage, Profils & Workflows Déterministes

Avant de charger un contexte large, sélectionner le profil adéquat et le workflow déterministe correspondant :

| Type de requête | Profil | Workflow | Compétences clés | Commande de validation |
|---|---|---|---|---|
| API & backend Rust | `rust-api` | `implement-feature` ou `fix-bug` | `rust`, `axum`, `sqlx-code-review`, `rust-testing` | `cargo check --workspace` |
| Interface React / Web | `react-web` | `implement-feature` ou `fix-bug` | `react`, `typescript`, `shadcn`, `frontend-design` | `pnpm check:web` (ou check ciblé) |
| Base de données & Migrations | `database` | `implement-feature` | `postgresql`, `sqlx-code-review`, `migrations` | Check API + revue migration |
| Audit de sécurité & secrets | `security` | `review-pr` | `security-expert`, `owasp`, `secrets-management` | `pnpm check:secrets` + findings |
| Architecture & Refactoring | `architect` | `refactor-module` | `clean-architecture`, `adr-drafting` | Plan / ADR + checks ciblés |
| Documentation & Agent | `docs` | `docs-update` | `typescript-docs`, `documentation-system` | `pnpm agent:doctor` |
| Requête large ou ambiguë | `manager` | `triage` | découverte ciblée | Cadrage explicite avant toute édition |

### Modes d'Opération de l'Agent
- **analysis** : Revue, plan, architecture (lecture seule privilégiée).
- **implementation** : Modification ciblée de code avec scope explicite.
- **review** : Analyse de bugs, régressions et tests (findings en premier, ordonnés par criticité).
- **refactor** : Redesign structurel sans introduire de dette technique temporaire.

### Contrat de Fin de Tâche (Definition of Done)
1. Fichiers modifiés clairement identifiés.
2. Commandes de validation exécutées et succès confirmé (ou justification explicite si un check est sauté).
3. Aucun warning ou erreur de compilation résiduelle.
4. Proposition de mise à jour de mémoire (`.agents/memory/`) uniquement si le savoir acquis est durable.

---

## 6. Gestion de la Mémoire de l'Agent (Agent Memory)

La mémoire persistante (`.agents/memory/`) stocke le savoir durable du projet, et non des notes de session éphémères.

| Emplacement | Rôle |
|---|---|
| `.agents/memory/project.md` | Synthèse stable du projet, direction V1 et périmètres actifs |
| `.agents/memory/architecture.md` | Décisions architecturales durables et justifications |
| `.agents/memory/conventions.md` | Conventions de code et règles non évidentes |
| `.agents/memory/gotchas.md` | Modes d'échec récurrents, pièges et bords tranchants |
| `.agents/memory/session-notes.md` | Notes temporaires à promouvoir ou purger |

### Ce qu'il faut persister
- Décisions architecturales clés.
- Commandes et gates de validation spécifiques au monorepo.
- Conventions non intuitives.
- Pièges de configuration ou d'outillage récurrents.

### Ce qu'il ne faut PAS persister
- Connaissances génériques de langages ou frameworks.
- Bugs ponctuels une fois résolus.
- Plans d'implémentation temporaires ou listes TODO périmées.
- Secrets, tokens, identifiants ou données personnelles.

*Règle de taille* : Maintenir chaque fichier mémoire sous la barre des **200 lignes**.

---

## 7. Compétences (Skills) & Intégrations MCP

- **Registre des compétences** : Déclaré dans `.agents/registry.yaml` avec implémentations dans `.agents/skills/<skill>/SKILL.md`.
- **Règle de chargement** : Charger le plus petit ensemble de compétences suffisant pour la tâche. Préférer les compétences spécifiques au repo aux compétences génériques.
- **Serveurs MCP configurés** :
  - `nx-mcp` (`npx nx mcp`) : Inspection du graphe de projets, dépendances et exécution des cibles Nx.
  - `shadcn` (`npx shadcn@latest mcp`) : Découverte et installation de composants d'interface.
- **Maintenance** : `pnpm agent:skills` et `pnpm agent:doctor`.

---

## 8. Conventions de Code & Architecture LLM-Friendly

### Limites de Taille de Fichier

| Seuil | Action |
|---|---|
| < 400 lignes | Contexte LLM optimal : lecture complète en un appel sans dilution |
| 400 - 800 lignes | **Avertissement (Warning)** : Envisager l'extraction (`scripts/check-llm-structure.sh`) |
| > 800 lignes | **Erreur bloquante stricte** : Obligation d'extraire |

### Conventions Rust (Flat Dot-Notation)

Les fichiers Rust utilisent une structure plate qualifiée par des points pour une navigation LLM immédiate :

```
apps/api/src/
├── api.config.rs            # Configuration fail-closed (lookup pur, testable)
├── api.identity.runtime.rs  # Secrets et registres Identity validés au démarrage
├── api.identity.rs          # Montage HTTP du domaine Identity
├── api.identity.activity.rs # Vérification d'activité des jetons in-process (Account)
├── api.client_ip.rs         # IP client derrière les proxies de confiance
└── api.routes.rs            # Assemblage du routeur
```

- Tous les `.rs` résident directement dans `src/` (aucun sous-répertoire profond).
- Pas de `utils.rs` ou `helpers.rs` génériques : chaque concept a son fichier nommé.
- Pas de logique métier dans les fichiers de barrel (`mod.rs`).

### Architecture des Services

- **Module-as-a-Service** : Si un service n'a pas d'état interne, privilégier des fonctions pures dans un module plutôt qu'une struct avec méthodes.
- **Séparation stricte des couches** : Maintenir une séparation nette entre persistance (`db.rs`), validation métier pure (`validation.rs`) et orchestration (`service.rs`).
- **Dépendances explicites** : Préférer passer les dépendances exactes (`&PgPool`, `&Config`) aux fonctions plutôt que l'objet global `&AppState` (évite le pattern Service Locator).
- **TypeScript strict** : INTERDICTION STRICTE du type `any`. Utiliser `unknown` ou des types précis et validés.

---

## 9. Hiérarchie de Sélection de Composants Frontend (Obligatoire)

Pour tout composant ou écran d'interface web, respecter STRICTEMENT cet ordre :

1. **Registry interne du projet** : Réutiliser les composants existants (`apps/*/components/ui`, local `components.json`).
2. **Registry officiel shadcn/ui** : Installer via `pnpm dlx shadcn@latest add <component>`.
3. **Registry externe compatible** : Composants accessibles tiers (ex: `@radix-ui/themes`, `@headlessui/react`).
4. **Tailwind CSS** : Styliser via classes utilitaires pures.
5. **CSS pur / modules** : En tout dernier recours exceptionnel uniquement.

### Règles d'Interface & Design
- **Interdiction du "fait maison"** : Tout composant custom doit être justifié par écrit (pourquoi les niveaux 1 à 4 ne conviennent pas).
- **Pas de cards imbriquées** : Une carte représente un bloc autonome. Pour structurer l'intérieur d'une carte, utiliser des sections, bordures, séparateurs ou fonds subtils.
- **Zéro emoji dans l'UI** : Utiliser des icônes SVG de bibliothèque (Lucide). Seul l'emoji cookie est toléré pour les bandeaux de consentement.

---

## 10. Règles de Qualité et Sécurité en Amont

Ces règles sont vérifiées localement avant commit (`pnpm check:upstream-rules`, `pnpm check:llm-quality`) :

### 1. SonarQube & SonarCloud
- **SQL (`plsql:DeleteOrUpdateWithoutWhereCheck`)** : Tout `UPDATE` ou `DELETE` SQL DOIT impérativement comporter une clause `WHERE` explicite.
- **TypeScript (`typescript:S2871`)** : Tout `.sort()` sur tableau DOIT fournir un comparateur explicite (ex: `(a, b) => a.localeCompare(b)` ou `(a, b) => a - b`).
- **Promises (`typescript:S6671`)** : Ne rejeter une Promise qu'avec une instance de `Error` (`reject(new Error(...))`).
- **Anti-ReDoS (`typescript:S8786`)** : Interdiction des quantificateurs imbriqués ou regex à risque de backtracking catastrophique.
- **Aleatoire sécurisé (`typescript:S2245`)** : Interdiction de `Math.random()` dans tout contexte de sécurité, sessions, tokens, secrets ou signaux bot. Utiliser exclusivement `crypto.getRandomValues()`.
- **Parsing numérique (`typescript:S7773`)** : Utiliser toujours `Number.parseInt()` et `Number.parseFloat()`.
- **Paramètres par défaut (`typescript:S7737`)** : Ne pas initialiser avec des objets mutables recalculés ; déclarer des constantes de module.
- **Rust asynchrone (`rust:S7487`)** : Interdiction formelle d'appels bloquants dans Tokio (`thread::sleep`, `child.wait()`). Utiliser `tokio::task::spawn_blocking` ou les équivalents asynchrones Tokio.
- **Couverture de code** : SonarCloud exige au minimum **80% de couverture** sur tout nouveau code. Chemins LCOV relatifs à la racine (`SF:apps/...`, `SF:libs/...`).
- **Duplication** : Moins de 3% de duplication sur le nouveau code.

### 2. CodeQL & Sécurité Applicative
- **Injection de commandes** : Ne jamais interpoler de variables dans une chaîne shell (`exec`). Utiliser `execFile` ou `spawn` avec un tableau d'arguments.
- **Path Traversal** : Toujours valider et confiner les chemins résolus via `path.resolve`.
- **GitHub Actions** : Ne JAMAIS interpoler `${{ github.event... }}` dans un bloc `run:`. Mapper systématiquement dans des variables d'environnement `env:`.
- **Rust Unsafe** : Tout bloc `unsafe` DOIT comporter un commentaire `// SAFETY:` explicitant ses garanties et invariants mémoire.

### 3. Contrôles de Sécurité CI
- **OSV Scanner** : Zéro CVE active dans `pnpm-lock.yaml` ou `Cargo.lock`.
- **Cargo Deny** : Licences autorisées (AGPL-3.0, MIT, Apache-2.0, BSD-3-Clause) et zéro advisory RUSTSEC.
- **Gitleaks** : Zéro secret, token, mot de passe ou clé privée dans le dépôt.
- **Trivy** : Zéro vulnérabilité ou misconfiguration de sévérité HIGH ou CRITICAL.
- **Zizmor** : Actions GitHub épinglées par SHA complet, permissions minimales `permissions: contents: read`.

### 4. Gates & Ratchets NVBES
- **Migrations DB** : Tout fichier de migration SQL comporte un `-- migrate:up` et un `-- migrate:down` symétriques et se termine par un saut de ligne.
- **Ratchets** : Maintien strict des ratchets (`unwrap-ratchet`, `sqlx-ratchet`, `weak-assertions`, `thresholds-monotone`).

---

## 11. Directives d'Orchestration Nx

- Préférer systématiquement l'exécution de tâches via Nx (`pnpm nx run`, `pnpm nx run-many`, `pnpm nx affected`).
- Préfixer les commandes avec `pnpm` (évite l'utilisation d'une CLI globale).
- Exploration du workspace : Utiliser en priorité la compétence `nx-workspace`.
- Génération & Scaffolding : Utiliser en priorité la compétence `nx-generate`.
- Ne jamais deviner de drapeaux CLI Nx : toujours vérifier avec `--help` ou `nx_docs`.

---

## 12. Commandes de Développement et Validation

```bash
# Runtimes Locaux
pnpm dev              # API (8080) + Worker (8081)
pnpm dev:api          # API Monolithe Modulaire en isolation
pnpm dev:worker       # Worker en isolation (`run migrate` puis `serve`)
pnpm dev:identity-web # Interface Identity hébergée (port 3000)
pnpm dev:account-web  # Interface Account (port 3001)
pnpm dev:infra        # Infrastructure locale (PostgreSQL, Valkey, Mailpit)

# Vérifications & Tests
pnpm check            # Suite complète (structure, limites, boundaries, secrets, finops, web, api)
pnpm check:llm-quality# Ratchets qualité (monotone, unwraps, sqlx, weak assertions, upstream rules)
pnpm check:structure  # Platitude src/ et seuils de taille (400 warning / 800 erreur)
pnpm lint             # Linter Biome & cargo clippy
pnpm format:check     # Vérification du formatage Biome & rustfmt
pnpm test             # Tests complets

# Ciblés Web & Rust
pnpm check:web        # Typecheck & linting TypeScript
cargo check --workspace # Compilation de l'ensemble du workspace Cargo

# Nx
pnpm nx:show-projects # Liste des projets du monorepo
pnpm nx affected -t check # Vérifie uniquement les projets affectés
```

---

## 13. Gotchas & Pièges Récurrents (Gotcha Memory)

- **Dirty worktrees** : Ne jamais exécuter de commandes de purge git (`git reset --hard`, `git checkout .`, `git clean -fd`) sans instruction explicite de l'utilisateur.
- **Ports locaux standardisés** :
  - `8080` : API Monolithe Modulaire
  - `8081` : Worker unifié d'arrière-plan
  - `3000` : Identity Web
  - `3001` : Account Web
  - `5432` : PostgreSQL
  - `6379` : Valkey / Redis
  - `8025` / `1025` : Mailpit (UI web / SMTP local)
- **Multi-schémas PostgreSQL** : chaque domaine migre dans son schéma (`identity`, `account`) via `nvbes-worker run migrate` ; l'API et le worker résolvent les tables par `search_path = identity, account, public`. L'API ne migre jamais.
- **Secrets partagés** : `NVBES_IDENTITY_MFA_ENCRYPTION_KEY` et `NVBES_IDENTITY_RATE_LIMIT_KEY` doivent être identiques sur toutes les instances API et le worker ; hors `development`/`test`, aucun secret n'est généré et l'absence d'une variable empêche le démarrage.
- **Variables d'environnement** : Toujours utiliser `.env` (synchronisé avec `.env.example` via `pnpm env:check`). Ne jamais commiter de secret réel.
