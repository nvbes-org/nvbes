#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

const driftErrors = [];

function readJson(filePath) {
  if (!existsSync(filePath)) return null;
  try {
    return JSON.parse(readFileSync(filePath, 'utf8'));
  } catch (error) {
    return null;
  }
}

function walkDir(dir, filterFn, results = []) {
  if (!existsSync(dir)) return results;
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== 'node_modules' && entry.name !== 'dist' && entry.name !== 'target' && !entry.name.startsWith('.')) {
        walkDir(fullPath, filterFn, results);
      }
    } else if (filterFn(fullPath)) {
      results.push(fullPath);
    }
  }
  return results;
}

export function checkDocumentationDrift() {
  console.log('🔎 Checking for documentation drift against live codebase...');

  const irPath = join(REPO_ROOT, '.docgen/workspace-ir.json');
  if (!existsSync(irPath)) {
    driftErrors.push('Missing workspace IR (.docgen/workspace-ir.json). Run "pnpm doc:extract" first.');
    reportResults();
    return;
  }

  const ir = JSON.parse(readFileSync(irPath, 'utf8'));

  // 1. Check for unindexed Nx projects
  const currentProjectFiles = walkDir(REPO_ROOT, (f) => f.endsWith('project.json'));
  for (const pFile of currentProjectFiles) {
    const pData = readJson(pFile);
    if (pData && pData.name) {
      const knownProject = ir.projects.find((p) => p.name === pData.name);
      if (!knownProject) {
        driftErrors.push(`New Nx project detected but missing in doc IR: "${pData.name}" (${relative(REPO_ROOT, pFile)})`);
      }
    }
  }

  // 2. Check for unindexed Rust crates
  const currentCargoFiles = walkDir(REPO_ROOT, (f) => f.endsWith('Cargo.toml'));
  for (const cFile of currentCargoFiles) {
    const rel = relative(REPO_ROOT, cFile);
    if (rel === 'Cargo.toml') continue;
    const content = readFileSync(cFile, 'utf8');
    const nameMatch = content.match(/\[package\][\s\S]*?name\s*=\s*"([^"]+)"/);
    if (nameMatch) {
      const crateName = nameMatch[1];
      const knownCrate = ir.rustCrates.find((c) => c.name === crateName);
      if (!knownCrate) {
        driftErrors.push(`New Cargo crate detected but missing in doc IR: "${crateName}" (${rel})`);
      }
    }
  }

  // 3. Check for unindexed OpenAPI specs
  const currentOpenApiFiles = walkDir(REPO_ROOT, (f) => f.endsWith('openapi.json') || f.endsWith('.openapi.json'));
  for (const oFile of currentOpenApiFiles) {
    const rel = relative(REPO_ROOT, oFile);
    const knownSpec = ir.openapiServices.find((o) => o.filePath === rel);
    if (!knownSpec) {
      driftErrors.push(`New OpenAPI spec file detected but missing in doc IR: "${rel}"`);
    }
  }

  // 4. Check for unindexed ADRs
  const currentAdrFiles = walkDir(join(REPO_ROOT, 'docs/adr'), (f) => f.endsWith('.md'));
  for (const adrFile of currentAdrFiles) {
    const filename = adrFile.split('/').pop();
    const knownAdr = ir.adrs.find((a) => a.filename === filename);
    if (!knownAdr) {
      driftErrors.push(`New ADR detected in docs/adr/ but missing in doc IR: "${filename}"`);
    }
  }

  reportResults();
}

function reportResults() {
  if (driftErrors.length > 0) {
    console.error('\n❌ Documentation Drift Detected! Code changes are out-of-sync with docs:');
    for (const err of driftErrors) {
      console.error(`  - ${err}`);
    }
    console.error('\n💡 To resolve, run: "pnpm doc:extract && pnpm doc:generate" to synchronize the documentation.\n');
    process.exit(1);
  }

  console.log('✅ Documentation is in 100% sync with the codebase (Zero drift detected)!');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  checkDocumentationDrift();
}
