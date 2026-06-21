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
