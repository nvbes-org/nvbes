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
