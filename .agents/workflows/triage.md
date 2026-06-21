# Triage Workflow

## Trigger

Use when a request is broad, ambiguous, or touches agent routing.

## Inputs

- User request.
- `AGENTS.md`.
- `docs/agent/agent.routing.md`.
- `.agents/registry.yaml`.

## Steps

1. Classify the request type.
2. Select one primary profile.
3. Select the smallest relevant skill set.
4. Select a workflow.
5. Define validation commands.
6. Ask only if a missing decision would make execution risky.

## Output Contract

- Scope.
- Profile.
- Workflow.
- Skills.
- Validation.
