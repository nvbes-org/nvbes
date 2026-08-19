#!/usr/bin/env node
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { extractWorkspaceIR } from './extract-workspace-ir.mjs';
import { generateDeterministicDocs } from './generate-deterministic-docs.mjs';
import { generateAllDomainDocs } from './generate-ai-doc.mjs';
import { generateAgentContext } from './generate-agent-context.mjs';
import { validateDocumentation } from './validate-docs.mjs';
import { checkDocumentationDrift } from './check-doc-drift.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

export function fixAllDocumentation() {
  console.log('🔧 Running full automated documentation synchronization & self-healing...\n');

  // 1. Extract IR
  extractWorkspaceIR();

  // 2. Generate Deterministic Catalogues, ADRs & API Pages
  generateDeterministicDocs();

  // 3. Synthesize Grounded Domain Architecture Docs
  generateAllDomainDocs();

  // 4. Generate Token-Optimized Agent Context Pack
  generateAgentContext();

  // 5. Run Quality Gates & Doc-Testing
  validateDocumentation();

  // 6. Verify Zero Drift
  checkDocumentationDrift();

  console.log('\n✨ Documentation system fully healed, synchronized, and validated!');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  fixAllDocumentation();
}
