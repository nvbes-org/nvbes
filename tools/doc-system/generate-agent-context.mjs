#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

export function generateAgentContext() {
  console.log('🤖 Generating token-optimized agent context pack...');

  const irPath = join(REPO_ROOT, '.docgen/workspace-ir.json');
  if (!existsSync(irPath)) {
    console.error('Workspace IR not found. Run "pnpm doc:extract" first.');
    process.exit(1);
  }

  const ir = JSON.parse(readFileSync(irPath, 'utf8'));

  const backendApps = ir.rustCrates.filter((c) => c.isApp);
  const webApps = ir.tsPackages.filter((p) => p.isApp);
  const rustLibs = ir.rustCrates.filter((c) => c.isLib);
  const tsLibs = ir.tsPackages.filter((p) => p.isLib);

  let doc = `# Monorepo nvbes - Agent Context Pack

> Fiche de contexte ultra-synthétique auto-générée pour agents IA (Cursor, Antigravity, Claude Code).
> Générée le : \`${ir.extractedAt}\`

## 1. Direction V1
- Livrer uniquement le socle Identity, Account, Billing, Email, Trust/Risk et Platform Operations.
- Cible B2C tout public avec équipes ; B2B, Enterprise, conformité avancée et Cloud/Drive sont futurs.
- Coût récurrent total : cible 20 EUR TTC, limite dure 30 EUR TTC.
- Un opérateur solo traite administration, support, abus et modération manuellement par défaut.
- Aucune dette technique ou structurelle connue dans le périmètre V1 livré.

## 2. Stack Technique
- **Backend** : Rust stable (Cargo workspace), Axum, SQLx, PostgreSQL serverless. Redis n'est pas une dépendance V1 active.
- **Frontend** : React 19, TypeScript, Vite, TanStack Router & Query, Tailwind CSS, shadcn/ui.
- **Monorepo Tooling** : pnpm workspaces, Nx, Biome/vp fmt, Lefthook git hooks.
- **Auth & Sécurité** : DPoP JWT (RFC 9449), WebAuthn/FIDO2, Cookies HTTP-only.

## 3. Services Backend actifs (Axum / Rust)
| Service | Chemin | Endpoints OpenAPI |
| :--- | :--- | :--- |
`;

  for (const svc of backendApps) {
    const openapi = ir.openapiServices.find(
      (o) => o.filePath.includes(svc.name) || o.filePath.includes(svc.path.split('/')[1]),
    );
    const count = openapi ? openapi.endpointsCount : '-';
    doc += `| \`${svc.name}\` | \`${svc.path}\` | ${count} endpoints |\n`;
  }

  doc += `\n## 4. Applications Frontend actives (React / Vite)
| Application | Chemin | Rôle |
| :--- | :--- | :--- |
`;

  for (const app of webApps) {
    doc += `| \`${app.name}\` | \`${app.path}\` | ${app.description || 'Interface utilisateur'} |\n`;
  }

  doc += `\n## 5. Bibliothèques disponibles
- **Rust (\`libs/rust/\`)** : ${rustLibs
    .slice(0, 10)
    .map((l) => `\`${l.name}\``)
    .join(', ')} (+${Math.max(0, rustLibs.length - 10)} autres).
- **TypeScript (\`libs/ts/\`)** : ${tsLibs.map((l) => `\`${l.name}\``).join(', ')}.

La présence d'une bibliothèque Cloud, Drive, Backoffice, Developer ou Enterprise ne la rend pas active. Vérifier la direction produit avant usage.

## 6. Règles Impératives du Codebase
1. **Fichiers < 300 lignes** : Tout fichier dépassant 300 lignes doit être découpé. Limite stricte à 500 lignes.
2. **Flat Dot-Notation (Rust)** : Tous les fichiers \`.rs\` sont à la racine de \`src/\` (ex: \`identity.domains.auth.service.rs\`).
3. **Zéro Dette Technique** : Pas de types \`any\` en TypeScript, pas de \`#[allow(unused)]\` sans justification en Rust.
4. **Pas de Service Locator** : Passer explicitement les dépendances (\`&PgPool\`, \`&Config\`) plutôt que l'objet global AppState.
5. **FinOps central** : Toute ressource ou automatisation doit tenir sous la limite globale de 30 EUR TTC.
6. **Opérations manuelles par défaut** : Automatiser uniquement un besoin mesuré, sûr, observable et réversible.

## 7. Commandes Rapides de Vérification
\`\`\`bash
pnpm check            # Suite complète de vérification monorepo
pnpm doc:validate     # Validation de la documentation (Mermaid, liens, snippets)
pnpm doc:check-drift  # Détection de désynchronisation code vs doc
pnpm dev:docs         # Surveiller et régénérer la documentation
\`\`\`
`;

  const lineCount = doc.split('\n').length;
  console.log(`📊 Generated context pack: ${lineCount} lines (limit: 300 lines).`);

  const outDir = join(REPO_ROOT, '.docgen');
  if (!existsSync(outDir)) mkdirSync(outDir, { recursive: true });

  const outPath = join(outDir, 'agent-context.md');
  writeFileSync(outPath, doc, 'utf8');
  console.log(`✅ Agent context pack saved to .docgen/agent-context.md`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  generateAgentContext();
}
