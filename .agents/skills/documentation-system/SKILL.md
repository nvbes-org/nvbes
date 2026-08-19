---
name: documentation-system
description: Guide for querying, maintaining, and auto-generating the nvbes documentation system. Use when adding new services, modifying API contracts, querying monorepo architecture, checking for documentation drift, or running doc validation.
---

# Documentation System Skill

Documentation architecture, auto-generation tooling, and quality guardrails in the `nvbes` monorepo.

## 1. Architecture du Système

Le système de documentation repose sur 3 piliers :
1. **Extraction Déterministe** : Extraction automatique de l'AST (`.docgen/workspace-ir.json`).
2. **Portail Développeur** : Application Astro Starlight (`apps/docs/`) avec recherche locale Pagefind.
3. **Quality Gates** : Validation continue de la syntaxe Mermaid, des liens et doc-testing des snippets.

## 2. Fichiers & Sources de Vérité
- **IR Monorepo** : `.docgen/workspace-ir.json` (Faits certifiés : projets Nx, crates Cargo, packages TS, endpoints OpenAPI).
- **Pack Contexte Agent** : `.docgen/agent-context.md` (Vue dense < 300 lignes de l'architecture).
- **ADRs** : `docs/adr/` (Source originale) → Synchronisé dans `apps/docs/src/content/docs/adr/`.
- **Spécifications OpenAPI** : `apps/*/openapi.json`.

## 3. Commandes Indispensables pour les Agents

Lors de modifications d'architecture, d'ajout d'endpoints ou de nouveaux packages :

```bash
# 1. Synchroniser et réparer toute la documentation en une seule commande
pnpm doc:fix

# 2. Vérifier l'absence de dérive entre le code et la doc
pnpm doc:check-drift

# 3. Valider la qualité de la doc (Mermaid + Liens + Doc-testing des snippets)
pnpm doc:validate

# 4. Compiler le portail et indexer dans Pagefind
pnpm build:docs
```

## 4. Règles de Rédaction de Documentation
- **Fichiers < 300 lignes** : Privilégier la concision technique sans superlatifs.
- **Syntaxe Mermaid stricte** : Toujours vérifier les fermetures d'accolades (`{}`) et les `subgraph ... end`.
- **Frontmatter obligatoire** : Tout fichier `.md` sous `apps/docs/src/content/docs/` doit contenir un en-tête YAML avec `title:` et `description:`.
- **Zéro Code Cassé** : Tout snippet JSON ou Shell doit être syntaxiquement valide.
