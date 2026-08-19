#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

/**
 * Zod / JSON Schema specification for LLM Documentation Generation.
 * Used to enforce strict output format from AI models.
 */
export const LLM_DOC_CONTRACTS = {
  serviceDoc: {
    type: 'object',
    required: ['title', 'summary', 'domain', 'architecturalRole', 'invariants', 'securityAndScopes', 'errorHandling', 'mermaidSequence'],
    properties: {
      title: { type: 'string', description: 'Titre explicite du document' },
      summary: { type: 'string', description: 'Résumé technique en 2-3 phrases sans superlatifs ni platitudes' },
      domain: { type: 'string', description: 'Nom du domaine métier (ex: identity, cloud, billing)' },
      architecturalRole: { type: 'string', description: 'Rôle architectural précis au sein du monorepo' },
      invariants: {
        type: 'array',
        items: { type: 'string' },
        description: 'Liste des règles métier strictes et invariables du service',
      },
      securityAndScopes: {
        type: 'object',
        required: ['authMethod', 'requiredScopes'],
        properties: {
          authMethod: { type: 'string', description: 'Mécanisme d authentification (ex: DPoP JWT, WebAuthn, API Key)' },
          requiredScopes: { type: 'array', items: { type: 'string' } },
        },
      },
      errorHandling: {
        type: 'array',
        items: {
          type: 'object',
          required: ['errorType', 'httpCode', 'recoveryStrategy'],
          properties: {
            errorType: { type: 'string' },
            httpCode: { type: 'number' },
            recoveryStrategy: { type: 'string' },
          },
        },
      },
      mermaidSequence: {
        type: 'string',
        description: 'Diagramme de séquence Mermaid syntaxiquement valide (sequenceDiagram) illustrant le flux principal',
      },
    },
  },
};

/**
 * Build grounded LLM prompt for a service, passing pure factual IR to eliminate hallucination.
 */
export function buildServiceDocPrompt(serviceName) {
  const irPath = join(REPO_ROOT, '.docgen/workspace-ir.json');
  if (!existsSync(irPath)) {
    throw new Error('Workspace IR not found. Please run extract-workspace-ir.mjs first.');
  }

  const ir = JSON.parse(readFileSync(irPath, 'utf8'));
  const project = ir.projects.find((p) => p.name === serviceName);
  const rustCrate = ir.rustCrates.find((c) => c.name === serviceName || c.path.includes(serviceName));
  const openapi = ir.openapiServices.find((o) => o.filePath.includes(serviceName));

  if (!project && !rustCrate) {
    throw new Error(`Service ${serviceName} not found in workspace IR.`);
  }

  const factualContext = {
    serviceName,
    rootPath: project?.root || rustCrate?.path,
    tags: project?.tags || [],
    internalDependencies: rustCrate?.internalDependencies || [],
    endpoints: openapi?.endpoints || [],
  };

  const systemPrompt = `Tu es un architecte logiciel expert chargé de rédiger la documentation technique du monorepo nvbes.

RÈGLES IMPÉRATIVES DE FIABILITÉ :
1. NE JAMAIS INVENTER de routes d'API, de bibliothèques, de dépendances ou de mécanismes non présents dans le CONTEXTE FACTUEL ci-dessous.
2. Tout diagramme Mermaid DOIT utiliser une syntaxe strictement valide.
3. Rédige en Markdown clair, concis, technique et actionnable pour des développeurs seniors.
4. Respecte la structure JSON demandée.`;

  const userPrompt = `Voici le contexte factuel certifié (extrait de l'AST et du graphe Nx) pour le service "${serviceName}" :

\`\`\`json
${JSON.stringify(factualContext, null, 2)}
\`\`\`

Génère la documentation structurée pour ce service en te basant STRICTEMENT sur ces données.`;

  return {
    systemPrompt,
    userPrompt,
    schema: LLM_DOC_CONTRACTS.serviceDoc,
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  console.log('LLM Contracts & Prompt Generator loaded.');
  if (process.argv[2]) {
    try {
      const prompt = buildServiceDocPrompt(process.argv[2]);
      console.log('\n--- Generated Prompt Preview ---');
      console.log(prompt.userPrompt);
    } catch (e) {
      console.error(e.message);
    }
  }
}
