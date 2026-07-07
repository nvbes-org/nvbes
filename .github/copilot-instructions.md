# nvbes repository instructions

## Project summary

nvbes is a full-stack monorepo with Rust APIs, React/Vite frontends, shared Rust crates, and TypeScript SDKs.

## Repository layout

- `apps/account-service`: Rust service for Account, identity, sessions, and token exchange
- `apps/cloud-service`: Rust service for Cloud workspaces and Drive surfaces
- `apps/billing-service`: Rust service for billing source-of-truth workflows
- `apps/developer-service`: Rust service for Developer Console APIs
- `apps/enterprise-service`: Rust service for enterprise governance workflows
- `apps/backoffice-service`: internal-only Backoffice service
- `apps/account-web`: React frontend for Account
- `apps/cloud-web`: React frontend for Cloud
- `apps/console-web`: React frontend for Developer Console
- `apps/enterprise-web`: React frontend for Enterprise
- `apps/backoffice-web`: React frontend for Backoffice
- `libs/rust/core`: shared Rust primitives
- `libs/ts/identity-sdk*`: TypeScript SDK packages
- `docs`: product, architecture, legal, testing, and operations docs

## Agent workflow

- Prefer the root scripts in `package.json` and the workspace Cargo commands.
- Inspect the smallest relevant set of files before editing.
- Keep changes scoped to the requested product area unless cross-cutting refactors are explicitly required.
- Do not introduce temporary compatibility shims unless the task explicitly asks for them.

## Rust rules

- Keep `src/` flat with dot-notation filenames.
- Use `#[path]` in module files to map flat files to Rust modules.
- Keep files under 300 lines when possible; above 500 lines, extract immediately.
- Avoid `utils.rs` and `helpers.rs`; give each concept its own file.
- Run `cargo check --workspace` after Rust changes.

## TypeScript and React rules

- Keep files short and focused.
- INTERDICTION STRICTE du type `any`. Utiliser `unknown` ou des types précis. Pas de passe-droit.
- Prefer explicit types for public APIs and shared helpers.
- Use existing patterns in the repo before introducing new abstractions.

## Documentation and validation

- Update docs when commands, structure, or conventions change.
- If logic changes, add a targeted test when it is practical.
- If a check cannot be run, explain why in the final response.
