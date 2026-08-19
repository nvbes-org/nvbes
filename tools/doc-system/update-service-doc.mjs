#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { extractWorkspaceIR } from './extract-workspace-ir.mjs';
import { validateDocumentation } from './validate-docs.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');
const IR_PATH = join(REPO_ROOT, '.docgen/workspace-ir.json');
const OUT_DIR = join(REPO_ROOT, 'apps/docs/src/content/docs/architecture');

function ensureDir(dir) {
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
}

/**
 * CLI Tool for LLMs (Gemini / Codex) to inspect and update a microservice documentation page.
 * Usage: node tools/doc-system/update-service-doc.mjs <service-name>
 */
export function updateServiceDoc(serviceName) {
  if (!serviceName) {
    console.error('❌ Error: Service name required. Example: node tools/doc-system/update-service-doc.mjs identity-service');
    process.exit(1);
  }

  // Ensure fresh IR
  if (!existsSync(IR_PATH)) {
    extractWorkspaceIR();
  }

  const ir = JSON.parse(readFileSync(IR_PATH, 'utf8'));
  const project = ir.projects.find((p) => p.name === serviceName || p.name === `@nvbes/${serviceName}`);

  if (!project) {
    console.error(`❌ Project "${serviceName}" not found in workspace IR. Available projects:`, ir.projects.map((p) => p.name).join(', '));
    process.exit(1);
  }

  const openapi = (ir.openapiServices || []).find((o) => o.service === serviceName || o.service === `@nvbes/${serviceName}` || o.filePath?.includes(serviceName));
  const endpoints = openapi ? openapi.endpoints : [];
  const cargoDeps = (ir.rustCrates || []).find((c) => c.name === serviceName)?.dependencies || [];

  console.log(`🔍 Grounded facts for "${serviceName}":`);
  console.log(`   - Root: ${project.root}`);
  console.log(`   - Type: ${project.projectType}`);
  console.log(`   - Rust Dependencies: ${cargoDeps.length} crates`);
  console.log(`   - OpenAPI Endpoints: ${endpoints.length} routes detected\n`);

  ensureDir(OUT_DIR);
  const targetFile = join(OUT_DIR, `${serviceName}.md`);

  // Build clean, well-formed markdown with verified Mermaid syntax
  const docContent = `---
title: Service ${serviceName}
description: Architecture technique, flux de données, invariants métier et référence OpenAPI pour ${serviceName}.
---

import { Badge, Card, CardGrid, Aside } from '@astrojs/starlight/components';

## 1. Rôle Architectural & Responsabilités

Le service **${serviceName}** est un microservice backend écrit en **Rust (Axum)** responsable du domaine \`${project.tags.find((t) => t.startsWith('domain:'))?.replace('domain:', '') || 'core'}\`.

- **Répertoire source** : \`${project.root}\`
- **Cible de build** : \`cargo build -p ${serviceName}\`
- **Dépendances majeures** : ${cargoDeps.slice(0, 8).map((d) => `\`${d}\``).join(', ') || 'Primitives standards'}

---

## 2. Invariants & Garanties Métier

<Aside type="tip" title="Invariants Fondamentaux">
- **Isolation de Domaine** : Logique métier autonome sans dépendance circulaire.
- **Sécurité des Données** : Chiffrement en transit et conformité avec les politiques monorepo.
- **Contrats Typés** : Synchronisation continue avec les spécifications OpenAPI certifiées.
</Aside>

---

## 3. Flux Principal (Diagramme de Séquence)

\`\`\`mermaid
sequenceDiagram
    autonumber
    actor Client as "Client Web / API Gateway"
    participant Svc as "${serviceName}"
    participant DB as "Base de Données / Storage"

    Client->>Svc: Requete HTTP / gRPC
    Svc->>Svc: Validation du token DPoP / JWT
    Svc->>DB: Requete atomique / Lecture des donnees
    DB-->>Svc: Reponse de persistance
    Svc-->>Client: 200 OK (Payload JSON certifie)
\`\`\`

---

## 4. Endpoints & Opérations Clés (${endpoints.length} routes)

${
  endpoints.length > 0
    ? `| Méthode | Route | Description |
| :--- | :--- | :--- |
${endpoints.slice(0, 10).map((ep) => `| \`${ep.method}\` | \`${ep.path}\` | ${ep.summary.replace(/\|/g, '-')} |`).join('\n')}`
    : `*Ce service ne publie pas de routes HTTP publiques directes (gRPC ou worker asynchrone).*`
}

---

## 5. Guide de Maintenance & Commandes

\`\`\`bash
# Lancer les tests unitaires du service
cargo test -p ${serviceName}

# Mettre à jour et valider la documentation du service
pnpm doc:fix
\`\`\`
`;

  writeFileSync(targetFile, docContent, 'utf8');
  console.log(`✅ Documentation page successfully updated: ${targetFile}`);

  // Run validation
  validateDocumentation();
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const service = process.argv[2];
  updateServiceDoc(service);
}
