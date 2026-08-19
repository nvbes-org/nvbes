#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

function readJson(filePath) {
  if (!existsSync(filePath)) return null;
  try {
    return JSON.parse(readFileSync(filePath, 'utf8'));
  } catch (error) {
    console.warn(`[WARN] Failed to parse JSON at ${filePath}: ${error.message}`);
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

export function extractWorkspaceIR() {
  console.log('🔍 Extracting deterministic monorepo facts (IR)...');

  // 1. Nx Projects
  const projectJsonFiles = walkDir(REPO_ROOT, (file) => file.endsWith('project.json'));
  const projects = [];

  for (const pFile of projectJsonFiles) {
    const data = readJson(pFile);
    if (!data || !data.name) continue;
    const relRoot = relative(REPO_ROOT, dirname(pFile));
    projects.push({
      name: data.name,
      projectType: data.projectType || 'unknown',
      root: relRoot,
      tags: Array.isArray(data.tags) ? data.tags : [],
      sourceRoot: data.sourceRoot || relRoot,
    });
  }

  // 2. TypeScript / Web packages
  const packageJsonFiles = walkDir(REPO_ROOT, (file) => file.endsWith('package.json'));
  const tsPackages = [];

  for (const pkgFile of packageJsonFiles) {
    const rel = relative(REPO_ROOT, pkgFile);
    if (rel === 'package.json') continue; // skip root
    const data = readJson(pkgFile);
    if (!data || !data.name) continue;

    const dir = dirname(pkgFile);
    const relDir = relative(REPO_ROOT, dir);

    tsPackages.push({
      name: data.name,
      version: data.version || '0.1.0',
      path: relDir,
      isApp: relDir.startsWith('apps/'),
      isLib: relDir.startsWith('libs/ts/'),
      dependencies: Object.keys(data.dependencies || {}),
      devDependencies: Object.keys(data.devDependencies || {}),
      scripts: Object.keys(data.scripts || {}),
      description: data.description || '',
    });
  }

  // 3. Rust Crates
  const cargoTomlFiles = walkDir(REPO_ROOT, (file) => file.endsWith('Cargo.toml'));
  const rustCrates = [];

  for (const cFile of cargoTomlFiles) {
    const rel = relative(REPO_ROOT, cFile);
    if (rel === 'Cargo.toml') continue; // skip root workspace Cargo.toml
    const content = readFileSync(cFile, 'utf8');
    const nameMatch = content.match(/\[package\][\s\S]*?name\s*=\s*"([^"]+)"/);
    const versionMatch = content.match(/version\s*=\s*"([^"]+)"/);

    if (nameMatch) {
      const dir = dirname(cFile);
      const relDir = relative(REPO_ROOT, dir);

      // Extract dependencies from Cargo.toml
      const depsSectionMatch = content.match(/\[dependencies\]([\s\S]*?)(\n\[|$)/);
      const internalDeps = [];
      if (depsSectionMatch) {
        const depsContent = depsSectionMatch[1];
        for (const line of depsContent.split('\n')) {
          const depMatch = line.match(/^([a-zA-Z0-9_-]+)\s*=/);
          if (depMatch && (line.includes('workspace = true') || line.includes('path = '))) {
            internalDeps.push(depMatch[1]);
          }
        }
      }

      rustCrates.push({
        name: nameMatch[1],
        version: versionMatch ? versionMatch[1] : '0.1.0',
        path: relDir,
        isApp: relDir.startsWith('apps/'),
        isLib: relDir.startsWith('libs/rust/'),
        internalDependencies: internalDeps,
      });
    }
  }

  // 4. OpenAPI Specs
  const openapiFiles = walkDir(REPO_ROOT, (file) => file.endsWith('openapi.json') || file.endsWith('.openapi.json'));
  const openapiServices = [];

  for (const oFile of openapiFiles) {
    const rel = relative(REPO_ROOT, oFile);
    const data = readJson(oFile);
    if (!data || !data.paths) continue;

    const endpoints = [];
    for (const [pathStr, methods] of Object.entries(data.paths)) {
      for (const [method, operation] of Object.entries(methods)) {
        if (typeof operation !== 'object' || !operation) continue;
        endpoints.push({
          path: pathStr,
          method: method.toUpperCase(),
          summary: operation.summary || operation.operationId || '',
          operationId: operation.operationId || '',
          tags: operation.tags || [],
          security: operation.security || [],
          responses: Object.keys(operation.responses || {}),
        });
      }
    }

    openapiServices.push({
      filePath: rel,
      title: data.info?.title || rel,
      version: data.info?.version || 'v1',
      description: data.info?.description || '',
      endpointsCount: endpoints.length,
      endpoints,
    });
  }

  // 5. ADRs
  const adrFiles = walkDir(join(REPO_ROOT, 'docs/adr'), (file) => file.endsWith('.md'));
  const adrs = [];

  for (const adrFile of adrFiles) {
    const rel = relative(REPO_ROOT, adrFile);
    const content = readFileSync(adrFile, 'utf8');
    const titleMatch = content.match(/^#\s+(.+)$/m);
    const statusMatch = content.match(/status:\s*(\w+)/i) || content.match(/\*\*Status:\*\*\s*(\w+)/i) || content.match(/##\s+Status\s*\n+([A-Za-z]+)/i);

    adrs.push({
      filePath: rel,
      filename: adrFile.split('/').pop(),
      title: titleMatch ? titleMatch[1].trim() : adrFile.split('/').pop(),
      status: statusMatch ? statusMatch[1].toLowerCase() : 'accepted',
    });
  }

  const ir = {
    extractedAt: new Date().toISOString(),
    monorepo: {
      totalProjects: projects.length,
      totalTsPackages: tsPackages.length,
      totalRustCrates: rustCrates.length,
      totalOpenApiServices: openapiServices.length,
      totalAdrs: adrs.length,
    },
    projects,
    tsPackages,
    rustCrates,
    openapiServices,
    adrs,
  };

  const outDir = join(REPO_ROOT, '.docgen');
  if (!existsSync(outDir)) {
    mkdirSync(outDir, { recursive: true });
  }

  const outPath = join(outDir, 'workspace-ir.json');
  writeFileSync(outPath, JSON.stringify(ir, null, 2), 'utf8');
  console.log(`✅ Intermediate Representation saved to ${relative(REPO_ROOT, outPath)}`);

  return ir;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  extractWorkspaceIR();
}
