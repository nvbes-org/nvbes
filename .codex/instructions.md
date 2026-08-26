# nvbes Agent Instructions

This file is generated from `docs/agent/*` by `pnpm agent:sync -- --write`.
Edit canonical files instead of editing this file directly.

<!-- source: docs/agent/agent.contract.md -->
# Agent Contract

This file is the canonical agent contract for nvbes. Tool-specific instruction
files must be generated from `docs/agent/*` and must not become independent
sources of truth.

## Operating Rules

- Read `AGENTS.md` before changing the repository.
- Treat `docs/product/nvbes-product-strategy.md` as the active V1 product
  authority. Cloud/Drive, B2B, Enterprise, and advanced compliance are not V1
  implementation scope.
- Keep the whole project's recurring cost at or below EUR 30 including taxes;
  FinOps is a design and delivery gate.
- Default to safe manual administration and moderation while there is one
  operator. Automate only a measured need with proven security and budget fit.
- Prefer short, flat, self-documenting files.
- Keep Rust files in `src/` with dot-qualified names and explicit `#[path]`
  module wiring.
- Keep TypeScript free of `any`; use precise types or `unknown`.
- Do not introduce known technical or structural debt in the delivered V1
  scope. After V1, record any accepted debt with an owner, measured impact,
  review date, and resolution condition.
- Do not rename or move files outside the requested scope.
- Use Nx to reduce context when project boundaries or affected scope matter.
- Use `rtk` as the shell command prefix.
- Use ripgrep for local discovery: `rg` for content searches and `rg --files`
  for file discovery. Fall back to `grep` or `find` only when `rg` is
  unavailable or unsuitable.
- Use `apply_patch` for manual file edits.
- Never revert user changes unless explicitly requested.

## Validation

- Rust changes require `cargo check --workspace`.
- Frontend or TypeScript changes require the targeted package check.
- Cross-cutting changes require relevant root checks.
- Business logic changes require a targeted test or an explicit reason why no
  test applies.

## Definition Of Done

- The selected profile, workflow, and skills are appropriate for the request.
- Context was loaded from the nearest applicable instructions.
- The implementation respects product and domain boundaries.
- Required checks were run or skipped with a concrete reason.
- Durable learnings were proposed for memory only when they improve future work.


<!-- source: docs/agent/agent.routing.md -->
# Agent Routing

Use this routing table before loading broad context. Select one primary profile,
then add only the skills and workflow required by the request.

| Request type | Profile | Workflow | Required skills | Validation |
|---|---|---|---|---|
| Rust API change | `rust-api` | `implement-feature` or `fix-bug` | `rust`, `axum`, `sqlx-code-review`, `rust-testing` | `cargo check --workspace` |
| React web change | `react-web` | `implement-feature` or `add-react-screen` | `react`, `typescript`, `shadcn`, `frontend-design` | targeted web check |
| Database or migration change | `database` | `implement-feature` | `postgresql`, `sqlx-code-review`, `migrations` | API check plus migration review |
| Security review | `security` | `review-pr` or `security-pass` | `security-review`, `owasp`, `secrets-management` | findings with file references |
| Architecture/refactor | `architect` | `refactor-module` or `architecture-review` | `agentic-architecture`, `repo-structure-playbook`, `adr-drafting` | ADR or plan plus scoped checks |
| Documentation or instructions | `docs` | `docs-update` | `memory-md-management`, `skill-authoring` | `pnpm agent:doctor` |
| Unknown or broad task | `manager` | `triage` | selected after discovery | explicit scope before edits |

## Control Model

- Prefer deterministic workflows for known tasks.
- Use autonomous exploration only for audits, architecture discovery, and broad
  debugging.
- Use subagents only when context isolation is useful enough to justify the
  overhead.
- Human approval is required for destructive, external, or irreversible actions.


<!-- source: docs/agent/agent.memory.md -->
# Agent Memory

Memory stores durable project knowledge, not session notes. It should make the
next agent faster and more accurate without bloating default context.

## Locations

| File | Purpose |
|---|---|
| `.agents/memory/project.md` | Stable project overview and boundaries |
| `.agents/memory/architecture.md` | Durable architecture decisions and rationale |
| `.agents/memory/conventions.md` | Non-obvious repo conventions |
| `.agents/memory/gotchas.md` | Recurring failure modes and sharp edges |
| `.agents/memory/session-notes.md` | Temporary notes to promote or delete |
| `.agents/memory/local.md` | Personal local preferences; gitignored if used |

## Persist

- Architecture decisions.
- Repo-specific commands and validation gates.
- Non-obvious conventions.
- Recurring failure modes.
- Tooling constraints that affect future agents.

## Do Not Persist

- Generic framework knowledge.
- One-off bugs after they are fixed.
- Temporary implementation plans.
- Stale TODO lists.
- Secrets, credentials, tokens, or personal data.

## Update Rule

At the end of substantial work, propose memory changes only when they are
durable. Keep each memory file under 200 lines where practical.


<!-- source: docs/agent/agent.skills.md -->
# Agent Skills

Skills are scoped operational instructions. They should route the model toward
the right project-specific behavior without loading broad reference material by
default.

## Registry

The project registry is `.agents/registry.yaml`. It maps request patterns to
skills, profiles, workflows, validation commands, and applicable paths.

## Skill Shape

```text
.agents/skills/<skill-name>/
├── SKILL.md
├── references/
└── scripts/
```

`SKILL.md` should stay below 500 lines and contain:

- frontmatter with a specific third-person description
- trigger terms
- required context
- one recommended workflow
- validation rules

Reference files are loaded only when the task needs them. Avoid nested reference
chains.

## Selection Rules

- Load the smallest set of skills that covers the request.
- Prefer project-specific skills over generic skills.
- Avoid stacking overlapping skills unless each adds a distinct constraint.
- If a named skill is missing or unclear, say so and continue with the closest
  safe fallback.

## Maintenance

Run:

```bash
pnpm agent:skills
pnpm agent:doctor
```


<!-- source: docs/agent/agent.workflows.md -->
# Agent Workflows

Workflows are deterministic task recipes. They live in `.agents/workflows` and
should be selected before edits begin.

## Required Sections

- Trigger
- Inputs
- Context discovery
- Steps
- Validation
- Output contract

## Core Workflows

| Workflow | Purpose |
|---|---|
| `triage` | classify request and select profile, skills, workflow |
| `implement-feature` | make a scoped feature change |
| `fix-bug` | reproduce, isolate, fix, verify |
| `refactor-module` | redesign bounded weak code without temporary debt |
| `review-pr` | code-review stance with findings first |
| `docs-update` | update instructions, memory, or docs |

## Rule

If a task has known steps, use a workflow. Use autonomous loops only when the
path is genuinely unknown.


<!-- source: docs/agent/agent.codex.md -->
# Codex Configuration

Codex is the implementation-oriented agent for this repo. It should operate from
the same canonical instructions as other tools.

## Defaults

- Read `AGENTS.md`.
- Use `rtk` for shell commands.
- Use ripgrep for local discovery: `rg` for content searches and `rg --files`
  for file discovery. Fall back to `grep` or `find` only when `rg` is
  unavailable or unsuitable.
- Use `apply_patch` for manual edits.
- Keep changes scoped and respect dirty worktrees.
- Do not use destructive git commands unless explicitly requested.
- Report checks run and checks skipped.

## Modes

| Mode | Use | Permissions |
|---|---|---|
| analysis | reviews, plans, architecture | read-only preferred |
| implementation | scoped code changes | workspace write |
| review | bugs, regressions, tests | read-only preferred |
| refactor | bounded redesign | workspace write with explicit scope |

## Output Contract

- Changed files.
- Validation commands and results.
- Skipped checks with reasons.
- Remaining risks or follow-up work.

## Delegation

When using Codex CLI from another tool, prompts must be in English and results
must be treated as untrusted guidance until reviewed.

