# @nvbes/identity-sdk-core

Spécification OpenAPI pour nvbes Identity permettant de générer des SDK cross-language.

## Structure

- `openapi.json` - Spécification OpenAPI 3.0 complète
- Génère automatiquement les types TypeScript et le client Rust

## Utilisation

### Génération TypeScript

```bash
pnpm generate:ts  # Crée src/types.gen.ts
```

### Génération Rust (via OpenAPI Generator)

```bash
pnpm generate:rust  # Génère dans identity-sdk-backend/
```

## SDK dérivés

- `@nvbes/identity-sdk-web` - Pour navigateurs (cookies HttpOnly)
- `nvbes-identity-sdk` (Rust) - Pour backends et services
