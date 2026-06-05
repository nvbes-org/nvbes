# nvbes repository instructions

## Project summary

nvbes is a full-stack monorepo with Rust APIs, React/Vite frontends, shared Rust crates, and TypeScript SDKs.

## Repository layout

- `apps/drive-api`: Rust API for Drive
- `apps/identity-api`: Rust API for Identity
- `apps/drive-web`: React frontend for Drive
- `apps/identity-web`: React frontend for Identity
- `crates/core`: shared Rust primitives
- `packages/identity-sdk*`: TypeScript and Rust SDKs
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
