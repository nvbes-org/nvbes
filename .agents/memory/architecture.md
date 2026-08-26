# Architecture Memory

## Agent Operating System

The repo uses `docs/agent/*` as canonical instructions. Tool-specific surfaces
such as `AGENTS.md`, `.codex/instructions.md`, and `antigravity.json` are synchronized
or aligned with canonical docs.

## Agent Topology

- Prefer deterministic workflows for known engineering tasks.
- Use manager routing to select profile, workflow, and skills.
- Use subagents only for context isolation on broad reviews or audits.

## Platform Boundaries

- Identity, Account, Billing, Email, Trust/Risk, and Platform Operations retain
  distinct ownership even when physical infrastructure is shared.
- The platform foundation must work without Cloud/Drive or any final product.
- Team primitives belong to the B2C Account boundary; Enterprise governance and
  compliance remain future extensions.
- FinOps, audit, outbox, idempotence, observability, and feature controls remain
  shared modules or contracts until measured evidence justifies extraction.
