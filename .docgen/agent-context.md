# Monorepo nvbes - Agent Context Pack

> Fiche de contexte ultra-synthétique auto-générée pour agents IA (Cursor, Antigravity, Claude Code).
> Générée le : `2026-08-26T09:36:01.861Z`

## 1. Direction V1
- Livrer uniquement le socle Identity, Account, Billing, Email, Trust/Risk et Platform Operations.
- Cible B2C tout public avec équipes ; B2B, Enterprise, conformité avancée et Cloud/Drive sont futurs.
- Coût récurrent total : cible 20 EUR TTC, limite dure 30 EUR TTC.
- Un opérateur solo traite administration, support, abus et modération manuellement par défaut.
- Aucune dette technique ou structurelle connue dans le périmètre V1 livré.

## 2. Stack Technique
- **Backend** : Rust stable (Cargo workspace), Axum, SQLx, PostgreSQL serverless. Redis n'est pas une dépendance V1 active.
- **Frontend** : React 19, TypeScript, Vite, TanStack Router & Query, Tailwind CSS, shadcn/ui.
- **Monorepo Tooling** : pnpm workspaces, Nx, Biome/vp fmt, Lefthook git hooks.
- **Auth & Sécurité** : DPoP JWT (RFC 9449), WebAuthn/FIDO2, Cookies HTTP-only.

## 3. Services Backend actifs (Axum / Rust)
| Service | Chemin | Endpoints OpenAPI |
| :--- | :--- | :--- |
| `nvbes-email-worker` | `apps/email-worker` | - endpoints |
| `nvbes-trust-risk-service` | `apps/trust-risk-service` | - endpoints |

## 4. Applications Frontend actives (React / Vite)
| Application | Chemin | Rôle |
| :--- | :--- | :--- |

## 5. Bibliothèques disponibles
- **Rust (`libs/rust/`)** : `nvbes-analytics-posthog`, `nvbes-email-scaleway`, `nvbes-audit`, `nvbes-billing`, `nvbes-core`, `nvbes-dpop`, `nvbes-email`, `nvbes-identity-sdk`, `nvbes-observability`, `nvbes-platform` (+13 autres).
- **TypeScript (`libs/ts/`)** : `@nvbes/account-client`, `@nvbes/backoffice-service-sdk-core`, `@nvbes/billing-client`, `@nvbes/email-ui`, `@nvbes/http-client`, `@nvbes/identity-client`, `@nvbes/identity-sdk`, `@nvbes/identity-sdk-core`, `@nvbes/identity-sdk-web`, `@nvbes/web-runtime`, `@nvbes/web-ui`.

La présence d'une bibliothèque Cloud, Drive, Backoffice, Developer ou Enterprise ne la rend pas active. Vérifier la direction produit avant usage.

## 6. Règles Impératives du Codebase
1. **Fichiers < 300 lignes** : Tout fichier dépassant 300 lignes doit être découpé. Limite stricte à 500 lignes.
2. **Flat Dot-Notation (Rust)** : Tous les fichiers `.rs` sont à la racine de `src/` (ex: `identity.domains.auth.service.rs`).
3. **Zéro Dette Technique** : Pas de types `any` en TypeScript, pas de `#[allow(unused)]` sans justification en Rust.
4. **Pas de Service Locator** : Passer explicitement les dépendances (`&PgPool`, `&Config`) plutôt que l'objet global AppState.
5. **FinOps central** : Toute ressource ou automatisation doit tenir sous la limite globale de 30 EUR TTC.
6. **Opérations manuelles par défaut** : Automatiser uniquement un besoin mesuré, sûr, observable et réversible.

## 7. Commandes Rapides de Vérification
```bash
pnpm check            # Suite complète de vérification monorepo
pnpm doc:validate     # Validation de la documentation (Mermaid, liens, snippets)
pnpm doc:check-drift  # Détection de désynchronisation code vs doc
pnpm dev:docs         # Surveiller et régénérer la documentation
```
