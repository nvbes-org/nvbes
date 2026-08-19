# Monorepo nvbes - Agent Context Pack

> Fiche de contexte ultra-synthétique auto-générée pour agents IA (Cursor, Antigravity, Claude Code).
> Générée le : `2026-08-19T22:51:12.120Z`

## 1. Stack Technique
- **Backend** : Rust stable (Cargo workspace), Axum, SQLx, PostgreSQL, Redis.
- **Frontend** : React 19, TypeScript, Vite, TanStack Router & Query, Tailwind CSS, shadcn/ui.
- **Monorepo Tooling** : pnpm workspaces, Nx, Biome/vp fmt, Lefthook git hooks.
- **Auth & Sécurité** : DPoP JWT (RFC 9449), WebAuthn/FIDO2, Cookies HTTP-only.

## 2. Services Backend (Axum / Rust)
| Service | Chemin | Endpoints OpenAPI |
| :--- | :--- | :--- |
| `nvbes-account-service` | `apps/account-service-next` | 20 endpoints |
| `nvbes-account-worker` | `apps/account-worker` | - endpoints |
| `nvbes-backoffice-service` | `apps/backoffice-service` | 58 endpoints |
| `nvbes-billing-service` | `apps/billing-service` | - endpoints |
| `nvbes-billing-worker` | `apps/billing-worker` | - endpoints |
| `nvbes-cloud-service` | `apps/cloud-service` | 17 endpoints |
| `nvbes-cloud-worker` | `apps/cloud-worker` | - endpoints |
| `nvbes-developer-service` | `apps/developer-service` | 38 endpoints |
| `nvbes-email-worker` | `apps/email-worker` | - endpoints |
| `nvbes-enterprise-service` | `apps/enterprise-service` | - endpoints |
| `nvbes-gateway-cloud` | `apps/gateway-cloud` | - endpoints |
| `nvbes-identity-service` | `apps/identity-service` | 69 endpoints |
| `nvbes-identity-worker` | `apps/identity-worker` | - endpoints |

## 3. Applications Frontend (React / Vite)
| Application | Chemin | Rôle |
| :--- | :--- | :--- |
| `nvbes-account-web` | `apps/account-web` | Interface utilisateur |
| `nvbes-backoffice-web` | `apps/backoffice-web` | Interface utilisateur |
| `nvbes-cloud-web` | `apps/cloud-web` | Interface utilisateur |
| `nvbes-console-web` | `apps/console-web` | Interface utilisateur |
| `@nvbes/docs` | `apps/docs` | Interface utilisateur |
| `nvbes-enterprise-web` | `apps/enterprise-web` | Interface utilisateur |
| `nvbes-identity-web` | `apps/identity-web` | Interface utilisateur |

## 4. Bibliothèques Partagées Clés
- **Rust (`libs/rust/`)** : `nvbes-analytics-posthog`, `nvbes-email-scaleway`, `nvbes-audit`, `nvbes-billing`, `nvbes-core`, `nvbes-dpop`, `nvbes-email`, `nvbes-identity-sdk`, `nvbes-observability`, `nvbes-platform` (+12 autres).
- **TypeScript (`libs/ts/`)** : `@nvbes/account-client`, `@nvbes/backoffice-service-sdk-core`, `@nvbes/billing-client`, `@nvbes/email-ui`, `@nvbes/http-client`, `@nvbes/identity-client`, `@nvbes/identity-sdk`, `@nvbes/identity-sdk-core`, `@nvbes/identity-sdk-web`, `@nvbes/web-runtime`, `@nvbes/web-ui`.

## 5. Règles Impératives du Codebase
1. **Fichiers < 300 lignes** : Tout fichier dépassant 300 lignes doit être découpé. Limite stricte à 500 lignes.
2. **Flat Dot-Notation (Rust)** : Tous les fichiers `.rs` sont à la racine de `src/` (ex: `identity.domains.auth.service.rs`).
3. **Zéro Dette Technique** : Pas de types `any` en TypeScript, pas de `#[allow(unused)]` sans justification en Rust.
4. **Pas de Service Locator** : Passer explicitement les dépendances (`&PgPool`, `&Config`) plutôt que l'objet global AppState.

## 6. Commandes Rapides de Vérification
```bash
pnpm check            # Suite complète de vérification monorepo
pnpm doc:validate     # Validation de la documentation (Mermaid, liens, snippets)
pnpm doc:check-drift  # Détection de désynchronisation code vs doc
pnpm dev:docs         # Lancer le portail de documentation
```
