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
