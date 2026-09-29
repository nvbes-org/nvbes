#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { extractWorkspaceIR } from './extract-workspace-ir.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

const OUT_SITE_DIR = join(REPO_ROOT, 'docs/generated');
const OUT_DOCS_DIR = join(OUT_SITE_DIR, 'catalogs');
const OUT_ADR_DIR = join(OUT_SITE_DIR, 'adr');
const OUT_API_DIR = join(OUT_SITE_DIR, 'api');
const OUT_PRODUCT_DIR = join(OUT_SITE_DIR, 'product');

export function isArchivedPath(path) {
  return path === 'archive' || path.startsWith('archive/');
}

export function isActiveRuntimePath(path) {
  return path.startsWith('apps/') && !path.startsWith('apps/docs/') && !isArchivedPath(path);
}

function ensureDir(dir) {
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
}

function resetGeneratedMarkdownDirectory(dir) {
  if (!existsSync(dir)) return;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isFile() && entry.name.endsWith('.md')) {
      rmSync(join(dir, entry.name));
    }
  }
}

function writeMarkdown(path, content) {
  writeFileSync(path, `${content.trimEnd()}\n`, 'utf8');
}

export function generateDeterministicDocs() {
  const irPath = join(REPO_ROOT, '.docgen/workspace-ir.json');
  let ir;
  if (existsSync(irPath)) {
    ir = JSON.parse(readFileSync(irPath, 'utf8'));
  } else {
    ir = extractWorkspaceIR();
  }

  resetGeneratedMarkdownDirectory(OUT_DOCS_DIR);
  resetGeneratedMarkdownDirectory(OUT_ADR_DIR);
  resetGeneratedMarkdownDirectory(OUT_API_DIR);
  ensureDir(OUT_DOCS_DIR);
  ensureDir(OUT_ADR_DIR);
  ensureDir(OUT_API_DIR);
  ensureDir(OUT_PRODUCT_DIR);

  console.log('⚡ Generating deterministic documentation files...');

  // 1. Services Catalog
  generateServicesCatalog(ir);

  // 2. Shared Libraries
  generateSharedLibraries(ir);

  // 3. Architecture Matrix
  generateArchitectureMatrix(ir);

  // 4. API Inventory
  generateApiInventory(ir);

  // 5. ADR Catalog & Sync
  generateAdrCatalog(ir);
  syncAdrs();

  // 6. API Documentation Pages
  generateApiPages(ir);

  // 7. Canonical V1 direction
  syncCanonicalDirection();

  console.log('✅ Deterministic documentation generated successfully in docs/generated/');
}

function generateServicesCatalog(ir) {
  const catalogApps = ir.projects.filter(
    (p) => p.tags.includes('type:app') || p.root.startsWith('apps/'),
  );
  const apps = catalogApps.filter((p) => !isArchivedPath(p.root));
  const archivedApps = catalogApps.filter((p) => isArchivedPath(p.root));

  let content = `---
title: Catalogue des Services & Applications
description: Inventaire auto-généré des runtimes actifs et des applications archivées du monorepo nvbes.
---

> Ce document est généré automatiquement de manière déterministe à partir de l'état réel du monorepo.
> Dernière mise à jour : \`${ir.extractedAt}\`

## Vue d'ensemble

Le runtime actif contient **${apps.length} applications et services**. Les projets sous \`archive/\` sont exclus de ce total et ne constituent pas une roadmap produit.

| Application / Service | Domaine | Scope | Type | Emplacement |
| :--- | :--- | :--- | :--- | :--- |
`;

  for (const app of apps) {
    const domain =
      app.tags.find((t) => t.startsWith('domain:'))?.replace('domain:', '') || 'général';
    const scope = app.tags.find((t) => t.startsWith('scope:'))?.replace('scope:', '') || 'internal';
    const isService = app.name.includes('-service') || app.name.includes('-worker');
    const typeLabel = isService ? '⚙️ Backend (Rust/Axum)' : '💻 Frontend (React/Vite)';

    content += `| **\`${app.name}\`** | \`${domain}\` | \`${scope}\` | ${typeLabel} | \`${app.root}\` |\n`;
  }

  content += `\n## Inventaire archivé\n\nLes **${archivedApps.length} projets** ci-dessous sont conservés comme historique. Ils ne sont ni déployables ni actifs sans une nouvelle décision produit et FinOps.\n\n| Projet archivé | Domaine | Emplacement |\n| :--- | :--- | :--- |\n`;

  for (const app of archivedApps) {
    const domain =
      app.tags.find((t) => t.startsWith('domain:'))?.replace('domain:', '') || 'général';
    content += `| \`${app.name}\` | \`${domain}\` | \`${app.root}\` |\n`;
  }

  content += `\n## Détail des Services Backend

`;

  const backendServices = ir.rustCrates.filter((c) => c.isApp);
  for (const svc of backendServices) {
    content += `### \`${svc.name}\`
- **Chemin** : \`${svc.path}\`
- **Version** : \`${svc.version}\`
- **Dépendances internes** : ${svc.internalDependencies.length > 0 ? svc.internalDependencies.map((d) => `\`${d}\``).join(', ') : '_Aucune_'}

`;
  }

  writeMarkdown(join(OUT_DOCS_DIR, 'services-catalog.md'), content);
}

function generateSharedLibraries(ir) {
  const tsLibs = ir.tsPackages.filter((p) => p.isLib);
  const rustLibs = ir.rustCrates.filter((c) => c.isLib);

  let content = `---
title: Bibliothèques Partagées (Rust & TypeScript)
description: Répertoire exhaustif des packages et crates partagés au sein du workspace.
---

> Ce document est auto-généré à partir de l'analyse des workspaces Cargo et pnpm.

## 🦀 Bibliothèques Rust (\`libs/rust/\`)

Total : **${rustLibs.length} crates partagées**

| Crate | Chemin | Version | Dépendances internes |
| :--- | :--- | :--- | :--- |
`;

  for (const rLib of rustLibs) {
    const deps =
      rLib.internalDependencies.length > 0
        ? rLib.internalDependencies.map((d) => `\`${d}\``).join(', ')
        : '-';
    content += `| **\`${rLib.name}\`** | \`${rLib.path}\` | \`${rLib.version}\` | ${deps} |\n`;
  }

  content += `\n## 🟦 Bibliothèques TypeScript / SDKs (\`libs/ts/\`)

Total : **${tsLibs.length} packages partagés**

| Package | Chemin | Version | Description / Rôle |
| :--- | :--- | :--- | :--- |
`;

  for (const tLib of tsLibs) {
    content += `| **\`${tLib.name}\`** | \`${tLib.path}\` | \`${tLib.version}\` | ${tLib.description || '-'} |\n`;
  }

  writeMarkdown(join(OUT_DOCS_DIR, 'shared-libraries.md'), content);
}

function generateArchitectureMatrix(ir) {
  let content = `---
title: Matrice d'Architecture & Dépendances
description: Graphe relationnel et frontières entre les services, workers et bibliothèques.
---

## Graphe de Dépendances Rust (Crates & Services)

Le diagramme ci-dessous illustre les liaisons déterministes entre les services applicatifs et les primitives partagées.

\`\`\`mermaid
flowchart TD
`;

  const backendApps = ir.rustCrates.filter((c) => c.isApp);
  const internalLibs = ir.rustCrates.filter((c) => c.isLib);

  content += `    subgraph Applications ["Services & Workers"]\n`;
  for (const app of backendApps) {
    const safeAppName = app.name.replace(/[^a-zA-Z0-9]/g, '_');
    content += `        ${safeAppName}["${app.name}"]\n`;
  }
  content += `    end\n\n`;

  content += `    subgraph Libraries ["Bibliotheques Partagees"]\n`;
  for (const lib of internalLibs) {
    const safeLibName = lib.name.replace(/[^a-zA-Z0-9]/g, '_');
    content += `        ${safeLibName}["${lib.name}"]\n`;
  }
  content += `    end\n\n`;

  for (const app of backendApps) {
    const safeAppName = app.name.replace(/[^a-zA-Z0-9]/g, '_');
    for (const dep of app.internalDependencies) {
      const safeDepName = dep.replace(/[^a-zA-Z0-9]/g, '_');
      content += `    ${safeAppName} --> ${safeDepName}\n`;
    }
  }

  content += `\`\`\`

## Règles d'Isolation des Domaines (Boundaries)

- **Isolation stricte** : Aucun service d'un domaine (ex: \`identity\`) ne peut importer directement la base de données d'un autre domaine (ex: \`cloud\`).
- **Communication inter-services** : Réalisée exclusivement via des contrats HTTP / OpenAPI typés ou des queues asynchrones.
- **Lib Core** : \`libs/rust/core\` doit rester neutre et sans logique métier spécifique.
`;

  writeMarkdown(join(OUT_DOCS_DIR, 'architecture-matrix.md'), content);
}

function generateApiInventory(ir) {
  const activeOpenApiServices = ir.openapiServices.filter((service) =>
    isActiveRuntimePath(service.filePath),
  );
  const archivedOpenApiCount = ir.openapiServices.length - activeOpenApiServices.length;
  let content = `---
title: Inventaire des APIs & Endpoints
description: Liste exhaustive des endpoints OpenAPI extraits des spécifications du projet.
---

> Ce catalogue est extrait directement des fichiers de spécification OpenAPI générés par les services Axum.

Total de services actifs documentés : **${activeOpenApiServices.length}**

Les **${archivedOpenApiCount} spécifications hors runtimes actifs** trouvées dans les archives, la documentation ou les SDK sont exclues : elles ne représentent pas des APIs V1 actives.

`;

  for (const svc of activeOpenApiServices) {
    content += `## ${svc.title} (\`${svc.version}\`)
- **Fichier source** : \`${svc.filePath}\`
- **Nombre d'endpoints** : **${svc.endpointsCount}**

| Méthode | Route | Description / OperationId | Tags |
| :--- | :--- | :--- | :--- |
`;

    for (const ep of svc.endpoints) {
      const tags = ep.tags.length > 0 ? ep.tags.map((t) => `\`${t}\``).join(', ') : '-';
      content += `| \`${ep.method}\` | \`${ep.path}\` | ${ep.summary || ep.operationId || '-'} | ${tags} |\n`;
    }

    content += '\n';
  }

  writeMarkdown(join(OUT_DOCS_DIR, 'api-inventory.md'), content);
}

function generateAdrCatalog(ir) {
  let content = `---
title: Catalogue des ADRs (Architecture Decision Records)
description: Journal des décisions d'architecture prises sur la plateforme nvbes.
---

Total : **${ir.adrs.length} décisions enregistrées**

| Fichier ADR | Titre | Statut |
| :--- | :--- | :--- |
`;

  for (const adr of ir.adrs) {
    const statusBadge =
      adr.status === 'accepted' || adr.status === 'accepte' ? '✅ Accepté' : `⚠️ ${adr.status}`;
    const cleanSlug = adr.filename.replace(/\.md$/, '');
    content += `| [\`${adr.filename}\`](/adr/${cleanSlug}/) | **${adr.title}** | ${statusBadge} |\n`;
  }

  content += `\n## Pourquoi des ADRs ?
Les ADRs formalisent les choix structurants (changement de base de données, intégration d'un PSP, refonte de l'authentification). Tout changement architectural majeur doit faire l'objet d'un nouvel ADR avant implémentation.
`;

  writeMarkdown(join(OUT_DOCS_DIR, 'adr-catalog.md'), content);
}

function escapeYamlDoubleQuoted(value) {
  return value
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/[\r\n]+/g, ' ');
}

function syncAdrs() {
  const adrSourceDir = join(REPO_ROOT, 'docs/adr');
  if (!existsSync(adrSourceDir)) return;

  const entries = readdirSync(adrSourceDir).filter((f) => f.endsWith('.md'));
  for (const file of entries) {
    const rawContent = readFileSync(join(adrSourceDir, file), 'utf8');
    const titleLine = rawContent.split('\n').find((l) => l.startsWith('# '));
    const title = titleLine ? titleLine.replace(/^#[ \t]+/, '').trim() : file.replace(/\.md$/, '');

    // Format content with valid frontmatter for Starlight
    let cleanedBody = rawContent;
    if (rawContent.startsWith('# ')) {
      // Remove first # header since Starlight renders title from frontmatter
      cleanedBody = rawContent.replace(/^#[ \t]+[^ \t\n][^\n]*\n+/, '');
    }

    // Fix relative links like [ADR 0003](0003-internal-billing-platform.md) -> (/adr/0003-internal-billing-platform/)
    cleanedBody = cleanedBody.replace(/\((00\d\d-[^)]+)\.md\)/g, '(/adr/$1/)');

    const formatted = `---
title: "${escapeYamlDoubleQuoted(title)}"
description: Architecture Decision Record - nvbes platform
---

${cleanedBody}`;

    writeMarkdown(join(OUT_ADR_DIR, file), formatted);
  }
}

function generateApiPages(ir) {
  // 1. Overview
  const overviewContent = `---
title: Standards & Conventions d'API
description: Normes de conception des APIs REST, authentification DPoP, pagination et gestion des erreurs.
---

## Authentification & Sécurité

Toutes les requêtes vers les APIs de production doivent inclure :
- **Authorization** : \`DPoP <access_token>\` ou \`Bearer <jwt>\`
- **DPoP Proof** : En-tête \`DPoP: <jwt_proof>\` certifiant la clé privée du client HTTP
- **X-Request-Id** : Identifiant UUID v4 unique pour le traçage distribué

## Format Normalisé des Erreurs

Les erreurs d'API suivent la structure normalisée suivante :

\`\`\`json
{
  "error": {
    "code": "INVALID_INPUT",
    "message": "Le champ email est requis et doit être valide.",
    "details": {
      "field": "email"
    }
  }
}
\`\`\`
`;

  writeMarkdown(join(OUT_API_DIR, 'overview.md'), overviewContent);

  // 2. Individual API pages per OpenAPI service
  for (const svc of ir.openapiServices.filter((service) => isActiveRuntimePath(service.filePath))) {
    const serviceSlug =
      svc.filePath.split('/')[1] || svc.title.toLowerCase().replace(/[^a-z0-9]+/g, '-');
    let svcContent = `---
title: API ${svc.title}
description: Contrats OpenAPI, routes, paramètres et schémas pour ${svc.title} (${svc.version}).
---

> Source : \`${svc.filePath}\` • Version : \`${svc.version}\` • Endpoints : **${svc.endpointsCount}**

## Endpoints Disponibles

`;

    for (const ep of svc.endpoints) {
      const securityBadge = ep.security.length > 0 ? '🔒 Sécurisé' : '🌐 Public';
      svcContent += `### \`${ep.method}\` \`${ep.path}\`
- **Description** : ${ep.summary || ep.operationId || 'Sans description'}
- **Sécurité** : ${securityBadge}
- **Tags** : ${ep.tags.length > 0 ? ep.tags.map((t) => `\`${t}\``).join(', ') : '-'}
- **Codes Retour** : ${ep.responses.map((r) => `\`${r}\``).join(', ')}

---
`;
    }

    writeMarkdown(join(OUT_API_DIR, `${serviceSlug}.md`), svcContent);
  }
}

function syncCanonicalDirection() {
  const strategySource = join(REPO_ROOT, 'docs/product/nvbes-product-strategy.md');
  const roadmapSource = join(REPO_ROOT, 'docs/roadmap.md');

  const strategyBody = readFileSync(strategySource, 'utf8')
    .replace(/^#[ \t]+[^ \t\n][^\n]*\n+/, '')
    .replace(
      /\]\(\.\.\/([^)]+)\)/g,
      '](' + 'https://github.com/nvbes-org/nvbes/blob/main/docs/$1)',
    );
  const roadmapBody = readFileSync(roadmapSource, 'utf8').replace(/^#[ \t]+[^ \t\n][^\n]*\n+/, '');

  writeMarkdown(
    join(OUT_PRODUCT_DIR, 'nvbes-product-strategy.md'),
    `---\ntitle: Direction produit nvbes V1\ndescription: Source de vérité du périmètre produit et opérationnel V1.\n---\n\n${strategyBody}`,
  );
  writeMarkdown(
    join(OUT_SITE_DIR, 'roadmap.md'),
    `---\ntitle: Roadmap nvbes\ndescription: Ordre de livraison et NO-GO du socle V1.\n---\n\n${roadmapBody}`,
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  generateDeterministicDocs();
}
