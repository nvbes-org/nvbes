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
