# Guide des Workflows & Prompts IA (Gemini & Codex)

Ce guide décrit les commandes et les modèles de prompts pour piloter la documentation automatisée du monorepo **nvbes** avec les assistants et agents LLM (**Gemini / Antigravity, Codex, Claude Code, Cursor**).

---

## 1. Commandes CLI pour Agents IA

Les agents peuvent exécuter directement ces commandes pour mettre à jour la documentation sans risque d'hallucination :

```bash
# 1. Mettre à jour la documentation d'un microservice spécifique
pnpm doc:update-service <nom-du-service>
# Exemple :
pnpm doc:update-service billing-service
pnpm doc:update-service developer-service

# 2. Synchronisation et auto-réparation complète de toute la documentation
pnpm doc:fix

# 3. Vérifier l'absence de dérive entre le code source et la documentation
pnpm doc:check-drift

# 4. Valider la qualité (syntaxe Mermaid, liens et doc-testing des snippets)
pnpm doc:validate

# 5. Compiler et indexer la documentation locale
pnpm build:docs
```

---

## 2. Modèles de Prompts pour Gemini & Codex

### Prompt 1 : "Documenter un nouveau service ou une nouvelle route"
> *"J'ai ajouté une nouvelle route `/api/v1/billing/checkout` dans `billing-service`. Extrais l'IR via `pnpm doc:extract`, mets à jour la documentation du service avec `pnpm doc:update-service billing-service` et vérifie qu'il n'y a aucune dérive avec `pnpm doc:check-drift`."*

### Prompt 2 : "Rédiger un nouvel ADR d'architecture"
> *"Rédige un nouvel ADR dans `docs/adr/` pour documenter la décision d'adopter le protocole DPoP pour les tokens JWT. Utilise le format standard (`Title`, `Status`, `Context`, `Decision`, `Consequences`), puis exécute `pnpm doc:fix` pour le synchroniser dans le portail développeur."*

### Prompt 3 : "Audit de conformité de la documentation"
> *"Lance `pnpm doc:validate` et `pnpm doc:check-drift`. Si des diagrammes Mermaid ou des snippets de code sont invalides, applique les corrections nécessaires et confirme que 100% des tests passent."*

---

## 3. Règles d'Ingénierie pour LLMs

1. **Faits certifiés (Grounding)** : Les LLMs doivent toujours interroger `.docgen/workspace-ir.json` pour vérifier les noms des crates et les routes OpenAPI réelles.
2. **Diagrammes Mermaid** : Tout acteur ou participant avec espaces doit être entouré de guillemets doubles (`participant DB as "PostgreSQL (Billing DB)"`).
3. **Format des fichiers** : Conserver les fichiers sous la limite de **300 lignes** pour garantir un contexte LLM optimal.
