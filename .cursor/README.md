# Cursor project config

Ce dossier contient la configuration Cursor versionnee pour le monorepo.

- `mcp.json`: serveurs MCP actives au niveau projet.
- `rules/*.mdc`: regles persistantes appliquees par Cursor Agent.

Regles actuelles:

- `core-workflow.mdc` (globale)
- `typescript-react.mdc` (TS/TSX dans `apps/` et `packages/`)
- `rust-backend.mdc` (Rust dans `apps/` et `crates/`)
