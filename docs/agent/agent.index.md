# Agent Index

Start here when working on agent behavior for nvbes.

Product and implementation scope comes from
[`docs/product/nvbes-product-strategy.md`](../product/nvbes-product-strategy.md).
Agent instructions must not infer Cloud/Drive, B2B, Enterprise or advanced
compliance work from older plans.

| Topic | File |
|---|---|
| Contract | `docs/agent/agent.contract.md` |
| Routing | `docs/agent/agent.routing.md` |
| Memory | `docs/agent/agent.memory.md` |
| Skills | `docs/agent/agent.skills.md` |
| Codex | `docs/agent/agent.codex.md` |
| Workflows | `docs/agent/agent.workflows.md` |

## Maintenance Commands

```bash
pnpm agent:sync
pnpm agent:skills
pnpm agent:research
pnpm agent:doctor
```
