---
name: ai-doc-system
description: Guide for LLMs (Gemini, Codex, Claude Code, Antigravity) to query, update, and auto-generate documentation in the nvbes monorepo. Use when creating/modifying services, updating API endpoints, editing ADRs, or synchronizing documentation with code changes.
---

# AI Documentation System & Workflow Guide

Ce skill fournit aux assistants et agents IA (**Gemini, Codex, Antigravity, Claude Code**) les instructions opératoires pour maintenir, générer et synchroniser la documentation technique du monorepo **nvbes** sans hallucination.

---

## 1. Principes Fondamentaux (Zero Hallucination)

1. **Toujours interroger l'AST / IR d'abord** :
   La base factuelle certifiée du monorepo réside dans `.docgen/workspace-ir.json`.
   Avant de répondre à une question d'architecture ou de documenter un service, lisez ce fichier pour obtenir les vrais noms de crates, les routes OpenAPI réelles et les dépendances exactes.
2. **Ne jamais inventer d'endpoints ou de types** :
   Les schémas OpenAPI sont dans `apps/<service>/openapi.json`.
3. **Respecter la syntaxe Mermaid stricte** :
   - Tout participant ou acteur avec espaces/parenthèses **doit** avoir des guillemets : `participant DB as "PostgreSQL (Billing DB)"`.
   - Chaque instruction de séquence doit être sur sa propre ligne avec saut de ligne (`\n`).

---

## 2. Commandes d'Automatisation LLM

Lorsqu'un LLM ou un développeur effectue des modifications dans le monorepo :

### Mettre à jour la documentation d'un microservice :
```bash
# Génère ou met à jour la fiche d'architecture d'un service à partir de son AST/OpenAPI
node tools/doc-system/update-service-doc.mjs <nom-du-service>
# Exemple:
node tools/doc-system/update-service-doc.mjs developer-service
```

### Synchronisation globale et auto-réparation :
```bash
# Extrait l'IR, régénère tous les catalogues, fiches de domaines, ADRs et valide l'ensemble
pnpm doc:fix
```

### Vérifier l'absence de dérive (Code vs Documentation) :
```bash
pnpm doc:check-drift
```

### Valider la qualité (Mermaid + Liens + Doc-Testing) :
```bash
pnpm doc:validate
```

---

## 3. Workflow de Modification pour LLMs

Quand un agent termine d'ajouter ou de modifier une route Rust / Axum :
1. Régénérer le schéma OpenAPI : `pnpm generate:openapi`
2. Mettre à jour la documentation du service : `node tools/doc-system/update-service-doc.mjs <nom-du-service>`
3. Valider l'intégrité de la documentation : `pnpm doc:validate`
4. Vérifier l'absence de dérive : `pnpm doc:check-drift`
