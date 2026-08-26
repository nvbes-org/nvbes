# Web UI Unification Implementation Plan

> **Status: future product UI plan, outside active V1.** Shared UI work is only
> justified by an active consumer after product selection. See the
> [active direction](../../product/nvbes-product-strategy.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a shared Radix-based web UI package aligned with the Identity visual system, then migrate `account-web` and `cloud-web` to consume shared primitives and a reusable multi-account switcher.

**Architecture:** Introduce `libs/ts/web-ui` as the source of truth for common theme tokens, common UI primitives, and account switching UI. Keep app-specific flows in each app via thin wrappers so product behavior remains local while the reusable surface moves to the shared package.

**Tech Stack:** React 19, Vite Plus, Tailwind CSS v4, Radix UI, Vitest, pnpm workspaces, Nx

---

### Task 1: Create the shared `web-ui` package skeleton

**Files:**
- Create: `libs/ts/web-ui/package.json`
- Create: `libs/ts/web-ui/project.json`
- Create: `libs/ts/web-ui/tsconfig.json`
- Create: `libs/ts/web-ui/vite.config.ts`
- Create: `libs/ts/web-ui/src/index.ts`

- [ ] Add the new workspace package and expose `index.ts` plus `styles.css`.
- [ ] Configure `vp pack`, `vp test`, and TypeScript paths consistent with other `libs/ts/*` packages.
- [ ] Ensure the package declares `react`, `radix-ui`, `lucide-react`, `class-variance-authority`, `clsx`, and `tailwind-merge`.

### Task 2: Lock shared switcher behavior with a failing test first

**Files:**
- Create: `libs/ts/web-ui/src/multi-account-switcher.test.ts`
- Create: `libs/ts/web-ui/src/multi-account-switcher.utils.ts`

- [ ] Write a failing test for active-account selection and initials generation.
- [ ] Run the targeted test and confirm it fails for missing implementation.
- [ ] Implement the minimal pure helpers to make the test pass.
- [ ] Re-run the targeted test and confirm it passes.

### Task 3: Add shared Identity-based theme and shared primitives

**Files:**
- Create: `libs/ts/web-ui/src/styles.css`
- Create: `libs/ts/web-ui/src/lib/classnames.ts`
- Create: `libs/ts/web-ui/src/components/ui/avatar.tsx`
- Create: `libs/ts/web-ui/src/components/ui/button.tsx`
- Create: `libs/ts/web-ui/src/components/ui/card.tsx`
- Create: `libs/ts/web-ui/src/components/ui/input.tsx`
- Create: `libs/ts/web-ui/src/components/ui/separator.tsx`
- Create: `libs/ts/web-ui/src/components/ui/sheet.tsx`
- Create: `libs/ts/web-ui/src/components/ui/skeleton.tsx`

- [ ] Copy the Identity visual tokens and base layer into shared package CSS.
- [ ] Move common primitive implementations into the shared package unchanged in API.
- [ ] Export all shared primitives from the package entrypoint.

### Task 4: Add the reusable multi-account switcher component

**Files:**
- Create: `libs/ts/web-ui/src/multi-account-switcher.tsx`

- [ ] Implement a generic account switcher that supports loading, active account state, optional remove action, and optional footer actions.
- [ ] Keep behavior generic through typed props instead of app-specific imports.

### Task 5: Migrate `account-web` to consume the shared package

**Files:**
- Modify: `apps/account-web/package.json`
- Modify: `apps/account-web/tsconfig.json`
- Modify: `apps/account-web/src/styles.css`
- Modify: `apps/account-web/src/components/AccountChooser.tsx`
- Modify: `apps/account-web/src/components/ui/avatar.tsx`
- Modify: `apps/account-web/src/components/ui/button.tsx`
- Modify: `apps/account-web/src/components/ui/card.tsx`
- Modify: `apps/account-web/src/components/ui/input.tsx`
- Modify: `apps/account-web/src/components/ui/separator.tsx`
- Modify: `apps/account-web/src/components/ui/sheet.tsx`
- Modify: `apps/account-web/src/components/ui/skeleton.tsx`

- [ ] Add `@nvbes/web-ui` as a workspace dependency and TS path alias.
- [ ] Replace the local common primitive implementations with re-exports from the shared package.
- [ ] Convert `AccountChooser` into a thin wrapper around the shared multi-account switcher.
- [ ] Replace local theme duplication by importing the shared stylesheet and keeping Identity-only animations local.

### Task 6: Migrate `cloud-web` to consume the shared package and Identity theme

**Files:**
- Modify: `apps/cloud-web/package.json`
- Modify: `apps/cloud-web/tsconfig.json`
- Modify: `apps/cloud-web/src/styles.css`
- Modify: `apps/cloud-web/src/DriveAccountMenu.tsx`
- Modify: `apps/cloud-web/src/components/ui/avatar.tsx`
- Modify: `apps/cloud-web/src/components/ui/button.tsx`
- Modify: `apps/cloud-web/src/components/ui/card.tsx`
- Modify: `apps/cloud-web/src/components/ui/input.tsx`
- Modify: `apps/cloud-web/src/components/ui/separator.tsx`
- Modify: `apps/cloud-web/src/components/ui/sheet.tsx`
- Modify: `apps/cloud-web/src/components/ui/skeleton.tsx`

- [ ] Add `@nvbes/web-ui` as a workspace dependency and TS path alias.
- [ ] Replace the local common primitive implementations with re-exports from the shared package.
- [ ] Replace the custom account menu UI with a thin wrapper around the shared switcher and keep Drive-only auth actions local.
- [ ] Replace the Drive theme tokens with the shared Identity-derived stylesheet.

### Task 7: Clean up dead local files and validate

**Files:**
- Delete: `apps/account-web/src/components/AccountChooser.shared.tsx` if unused
- Delete: Drive account-menu view fragments if unused after wrapper migration
- Modify: root `package.json` checks to include `web-ui`

- [ ] Remove stale local component files once imports no longer reference them.
- [ ] Add `web-ui` to root workspace build/check/lint/typecheck scripts.
- [ ] Run targeted tests and typechecks for `web-ui`, `account-web`, and `cloud-web`.
