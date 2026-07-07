#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const manifestPath = 'tools/oss-export/manifest.json';
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const errors = [];

function normalizePath(value) {
  return value.replaceAll('\\', '/').replace(/\/+$/, '');
}

function hasGlob(value) {
  return value.includes('*');
}

function isCoveredByExclude(path) {
  const normalized = normalizePath(path);
  return manifest.exclude.some((entry) => {
    const exclude = normalizePath(entry);
    if (hasGlob(exclude)) {
      const prefix = exclude.slice(0, exclude.indexOf('*'));
      return normalized.startsWith(prefix);
    }
    return normalized === exclude || normalized.startsWith(`${exclude}/`);
  });
}

for (const required of ['include', 'exclude', 'copy']) {
  if (!Array.isArray(manifest[required])) {
    errors.push(`${manifestPath}: ${required} must be an array`);
  }
}

for (const source of manifest.include ?? []) {
  if (isCoveredByExclude(source)) {
    errors.push(`${source} is both included and excluded`);
  }

  if (!hasGlob(source) && !existsSync(source)) {
    errors.push(`${source} is listed in include but does not exist`);
  }
}

for (const mapping of manifest.copy ?? []) {
  if (!manifest.include.includes(mapping.from)) {
    errors.push(`${mapping.from} is copied but not listed in include`);
  }

  if (isCoveredByExclude(mapping.to)) {
    errors.push(`${mapping.to} is a public export target covered by exclude`);
  }
}

const requiredPrivateExcludes = [
  'docs/cloud',
  'docs/internal',
  'docs/blueprint',
  'deploy/cloud',
  'deploy/internal',
  'libs/ts/backoffice-service-sdk-core',
  'apps/backoffice-*',
  'apps/internal-*',
];

for (const privatePath of requiredPrivateExcludes) {
  if (!manifest.exclude.includes(privatePath)) {
    errors.push(`${privatePath} must stay excluded from OSS export`);
  }
}

function scanMarkdownDir(dir) {
  if (!existsSync(dir)) return;
  const stack = [dir];
  while (stack.length > 0) {
    const current = stack.pop();
    const stat = statSync(current);
    if (stat.isDirectory()) {
      for (const entry of readdirSync(current)) {
        stack.push(join(current, entry));
      }
      continue;
    }

    if (!current.endsWith('.md')) continue;
    const content = readFileSync(current, 'utf8');
    if (content.includes('docs/blueprint') || content.includes('../blueprint') || content.includes('blueprint/')) {
      errors.push(`${relative(process.cwd(), current)} must not link to private blueprint docs`);
    }
  }
}

scanMarkdownDir('docs/oss');

if (errors.length > 0) {
  console.error('OSS export checks failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log('OSS export manifest: ok');
