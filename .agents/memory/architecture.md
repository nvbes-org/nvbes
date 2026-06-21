# Architecture Memory

## Agent Operating System

The repo uses `docs/agent/*` as canonical instructions. Tool-specific surfaces
such as `AGENTS.md`, `.codex/instructions.md`, Cursor rules, and ChatGPT
project prompts should be generated or synchronized from canonical docs.

## Agent Topology

- Prefer deterministic workflows for known engineering tasks.
- Use manager routing to select profile, workflow, and skills.
- Use subagents only for context isolation on broad reviews or audits.
