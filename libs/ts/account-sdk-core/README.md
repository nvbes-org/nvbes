# @nvbes/account-sdk-core

Spécification OpenAPI pour nvbes Account permettant de générer des SDK
cross-language.

## Structure

- `openapi.json` — contrat HTTP Account V1
- `src/types.gen.ts` — types TypeScript générés

## Utilisation

```bash
pnpm generate:ts
```

Audience JWT attendue : `nvbes-account-service`.
Scopes : `account:read`, `account:write`, `account:export`, `account:close`.
