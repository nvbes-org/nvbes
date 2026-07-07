# nvbes Monorepo Agentic Refactor Workplan

Status: working draft
Owner: Codex

## Goal

Refactor nvbes into an Nx-oriented monorepo with explicit project boundaries, small files, and clear agent-friendly instructions.

## Operating Principles

- Treat every app and library as a first-class Nx project.
- Keep dependencies directional and explicit.
- Prefer domain boundaries over technical layers when splitting code.
- Keep Rust sources flat with dot-notation filenames.
- Avoid helper/util dumping grounds.
- Keep file sizes under the documented thresholds.
- Use Nx `affected` to minimize the blast radius of changes.

## Target Workspace Shape

```text
nvbes/
├── apps/
│   ├── account-service/
│   ├── cloud-service/
│   ├── account-web/
│   ├── cloud-web/
│   ├── account-worker/
│   └── cloud-worker/
├── libs/
│   ├── rust/
│   │   ├── core/
│   │   ├── audit/
│   │   ├── billing/
│   │   ├── email/
│   │   ├── observability/
│   │   ├── region/
│   │   ├── scan/
│   │   ├── storage/
│   │   ├── tenancy/
│   │   └── identity-sdk-backend/
│   └── ts/
│       ├── email-templates/
│       ├── http-client/
│       ├── identity-client/
│       ├── identity-sdk/
│       ├── identity-sdk-core/
│       ├── identity-sdk-web/
│       └── web-runtime/
├── scripts/
├── docs/
└── infra/
```

## Refactor Phases

### Phase 0 - Governance

- Consolidate repo rules in `AGENTS.md`.
- Keep `.github/copilot-instructions.md` aligned with the repo rules.
- Keep `.cursor/rules/*` consistent with the root instructions.
- Document architecture, conventions, and project targets.

### Phase 1 - Nx Graph

- Normalize `project.json` files.
- Define tags and dependency boundaries.
- Standardize `build`, `check`, `test`, `lint`, and `dev` targets.
- Make `affected` the default mental model for validation.

### Phase 2 - Shared Libraries

- [x] Stabilize `libs/core`.
- [x] Extract `libs/security`.
- [x] Extract `libs/observability`.
- [x] Extract `libs/auth`.
- [x] Extract `libs/authz`.
- [x] Extract `libs/tenancy`.
- [x] Extract `libs/billing`.
- [x] Completed physical restructure: moved `crates/` to `libs/rust/` and `packages/` to `libs/ts/`.
- [x] Extracted `identity.domains.oauth.service.types.rs` as the first step of oversized service decomposition.

### Phase 3 - Account service

- Keep the `src/` tree flat.
- Split oversized services.
- Move shared primitives into libraries.
- [x] Standardize auth helpers and use `nvbes_core::auth::helpers`.
- Preserve public HTTP contracts.

### Phase 4 - Cloud service

- [x] Keep the `src/` tree flat.
- [x] Split oversized services.
- [x] Reuse shared libraries.
- [x] Standardize auth helpers and use `nvbes_core::auth::helpers`.
- [x] Reduce duplication with identity.

### Phase 5 - Frontends

- Split UI shell from reusable UI.
- Move shared types and clients into SDK libraries.
- Keep presentation components thin.

### Phase 6 - CI and Checks

- Run targeted checks based on Nx affected scope.
- Keep structural checks for file size and layout.
- Fail fast on dependency boundary violations.

### Phase 7 - Cleanup

- Remove redundant exports.
- Rename ambiguous files.
- Delete leftover helper catch-alls.
- Re-check size thresholds everywhere.

## Execution Backlog

### Ticket 0 - Freeze the frame

- Align repo documentation.
- Link the workplan from the root docs.
- Ensure the rules mention Nx and the refactor direction.

### Ticket 1 - Nx workspace

- Finish workspace metadata.
- Verify project graph visibility.
- Confirm root commands.

### Ticket 2 - Shared core libraries

- Split the generic primitives into their own libraries.
- Keep app code from re-owning reusable logic.

### Ticket 3 - Account service split

- Decompose the largest domain services first (Targeting >500 line limit).
- [x] Extract `OAuthService` types to `.types.rs`.
- [x] Extract `FederationService` logic into sub-modules (OIDC, SAML, SCIM, etc.).
- [x] **Split `OAuthService` (2151 lines)**:
  - [x] Extract `flows.rs` (Auth code/Token).
  - [x] Extract `device.rs` (Device flow).
  - [x] Extract `policies.rs` (Client policies).
  - [x] Extract `clients.rs` (Client management).
  - [x] Extract `assurance.rs` (ACR/AMR logic).
- [x] **Split `BillingService` (1571 lines)**:
  - [x] Extract `stripe.rs` (Stripe client).
  - [x] Extract `webhooks.rs` (Webhook handling).
  - [x] Extract `entitlements.rs` (Plan/Feature checks).
- [x] **Split `AuthService` (1255 lines)**:
  - [x] Extract `credentials.rs` (Passwords/Reset).
  - [x] Extract `mfa.rs` (Move existing MFA logic if needed).
  - [x] Extract `sessions.rs` (Session management logic).
  - [x] Extract `verification.rs`, `onboarding.rs`, `risk.rs`.
- [x] **Cleanup `identity.authz.rs` (807 lines)**:
  - [x] Move to `identity.domains.authz.service.rs`.
  - [x] Split into `evaluator.rs` and `db.rs`.
- [x] **Split `MembersService` (744 lines)** and `WorkspacesService` (540 lines).
- [x] Split oversized `routes.rs` files into smaller functional sub-routers.
- [x] Replace local helpers with shared libraries where appropriate.

### Ticket 4 - Cloud service split

- [x] Apply the same decomposition pattern as Account.
- [x] Rename files to follow `drive.*` dot-notation.
- [x] Remove duplicated domain behavior.

### Ticket 5 - Frontend decomposition

- [x] Keep UI generated locally in each app with shadcn CLI and keep shared SDK layers in `libs/ts/sdk-*`.
- [x] Keep apps focused on page composition and routing.

### Ticket 6 - CI enforcement

- [x] Enforce project graph usage.
- [x] Enforce size and boundary rules (ensure `scripts/check-llm-structure.sh` is active).
- [x] Use affected-based checks for speed.

### Ticket 7 - Cleanup

- Remove redundant exports.
- Rename ambiguous files.
- Delete leftover helper catch-alls.
- Re-check size thresholds everywhere.

## Validation Rules

- Rust modifications must pass `cargo check --workspace`.
- Frontend modifications must pass the relevant package checks.
- Structural rules must pass before merge.
- Cross-domain dependency violations are blockers.

## Open Questions

- **Answered**: `tenancy` and `billing` now live as dedicated crates in `libs/rust/`. They should be moved to `libs/rust/` during the physical restructure. (Done)
- **Direction**: `billing` should be kept as a single crate for V1 but internally partitioned between domain logic and Stripe specifics. (Done)
- **Direction**: `identity-sdk-*` should be regrouped under `libs/ts/identity-sdk/` as sub-packages or a unified entry point to reduce workspace noise. (Done)
- **Next Step**: Start with `Ticket 7 - Cleanup` to finalize the refactor.
