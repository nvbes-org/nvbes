---
title: Guide - Créer un Nouveau Service Rust
description: Procédure pas à pas pour scaffolder, configurer et documenter un nouveau microservice Rust/Axum.
---

## 1. Création du Crate Rust

Créez le dossier du service dans `apps/<nom>-service/` et définissez le `Cargo.toml` :

```toml
[package]
name = "nvbes-<nom>-service"
version = "0.1.0"
edition = "2021"

[dependencies]
nvbes-core = { workspace = true }
nvbes-observability = { workspace = true }
axum = { workspace = true }
tokio = { workspace = true, features = ["full"] }
serde = { workspace = true, features = ["derive"] }
```

## 2. Déclaration Nx (`project.json`)

Ajoutez le fichier `apps/<nom>-service/project.json` avec les tags de frontières requis :

```json
{
  "name": "<nom>-service",
  "projectType": "application",
  "root": "apps/<nom>-service",
  "sourceRoot": "apps/<nom>-service/src",
  "tags": ["scope:internal", "type:app", "domain:<domaine>", "layer:app"]
}
```

## 3. Structure des Fichiers Rust (Conventions Flat Dot-Notation)

Respectez l'organisation plate sous `src/` :

```text
apps/<nom>-service/src/
├── main.rs
├── <nom>.app.rs                    # AppState et initialisation
├── <nom>.domains.<module>.mod.rs   # Déclaration des modules
├── <nom>.domains.<module>.service.rs # Logique métier
├── <nom>.domains.<module>.routes.rs  # Handlers HTTP Axum
└── <nom>.http.error.rs             # Gestion des erreurs
```

## 4. Génération de la Documentation & Spécification OpenAPI

Implémentez l'option `--export-openapi` dans votre service et mettez à jour `scripts/generate-openapi.sh`.

Ensuite, exécutez la génération de la documentation :

```bash
pnpm doc:extract
pnpm doc:generate
pnpm doc:validate
```
Le service et ses routes apparaîtront automatiquement dans le portail de documentation développeur.
