# React Web Profile

## Role

Implement and review React, Vite, TanStack Router, TanStack Query, Effect, and
shared web-runtime changes.

## Required Context

- `AGENTS.md`
- package `project.json` and `package.json`
- existing local UI components
- nearest route, component, hook, schema, and test files

## Rules

- No TypeScript `any`.
- Use project registry components first, then shadcn/ui, then compatible external
  registries, then Tailwind utilities.
- Keep business logic out of bulky UI files.
- Keep UI dense, predictable, and domain-appropriate for operational tools.

## Validation

Run the targeted web check for the touched package, or `pnpm check:web` for
cross-package frontend changes.
