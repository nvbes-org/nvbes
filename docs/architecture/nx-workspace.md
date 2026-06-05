# Nx Workspace and Dependency Graph

This document defines the Nx-oriented organization of nvbes.

## Purpose

Nx is used as the project graph and task orchestration layer on top of pnpm and Cargo.

It is responsible for:

- project discovery;
- target execution;
- dependency visibility;
- affected-based validation;
- cacheable task boundaries.

## Project Taxonomy

### Apps

- `type:app`
- `layer:app`

### Shared Core Libraries

- `type:lib`
- `layer:core`

### SDK Libraries

- `type:lib`
- `layer:sdk`

## Domain Tags

- `domain:identity`
- `domain:drive`
- `domain:shared`

## Dependency Rules

- Apps may depend on libs.
- Libs must not depend on apps.
- Domain-specific projects should not cross-import without a shared contract.
- Shared libraries should remain lower in the graph than app-specific code.
- A dependency that cannot be explained by the graph is a design smell.
- `domain:identity` and `domain:drive` must never depend on each other directly.
- `layer:core` projects are leaf primitives and must not depend on app projects.
- `layer:sdk` projects may depend on `layer:core` or their own domain, but not on apps.
- `domain:shared` projects must not depend on domain-specific projects.

## Enforced Check

The repo ships with `pnpm check:nx-boundaries`, which validates the current Nx graph against the rules above.

If the graph becomes invalid, the check fails before the issue can reach CI.

## Standard Targets

Every project should expose the same conceptual target set where applicable:

- `build`
- `check`
- `test`
- `lint`
- `dev`

Targets should be named consistently even if their executors differ by stack.

## Workflow

1. Inspect the project graph with `pnpm nx:show-projects`.
2. Inspect a single project with `pnpm nx:show-project <project>`.
3. Determine blast radius with `pnpm nx:affected`.
4. Implement the smallest change that satisfies the target.
5. Validate only the affected projects first.
6. Run workspace-wide validation only when the change is cross-cutting.

## Notes

- Nx does not replace Cargo or pnpm.
- Nx describes and orchestrates the workspace.
- Rust and TypeScript still keep their native build/test commands.
