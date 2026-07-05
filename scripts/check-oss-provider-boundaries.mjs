#!/usr/bin/env node
import { existsSync, lstatSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const policyPath = 'scripts/oss-provider-baseline.json';
const policy = JSON.parse(readFileSync(policyPath, 'utf8'));
const knownProviderFiles = new Set(policy.knownProviderFiles);
const policyFiles = new Set([policyPath, 'scripts/check-oss-provider-boundaries.mjs']);
const forbiddenIdentityBillingBaselinePatterns = [
  /^apps\/identity-api\/src\/identity\.domains\.billing(?:\.|$)/,
  /^apps\/identity-api\/migrations\/(?:0016_billing_platform_core|0017_regional_price_mappings|0020_internal_admin_entitlement_actions|0022_internal_admin_usage_actions|0027_internal_admin_risk_actions|0028_internal_admin_operations_actions|0029_internal_admin_revenue_actions|0030_internal_admin_billing_platform_actions)\.sql$/,
];
const blockedTerms = [
  'stripe',
  'sentry',
  'scaleway',
  'cloudflare',
  'posthog',
  'sendgrid',
  'mailgun',
  'postmark',
];
const skippedDirs = new Set(['.git', '.nx', 'coverage', 'dist', 'node_modules', 'target']);
const textExtensions = new Set([
  '.cjs',
  '.css',
  '.html',
  '.json',
  '.js',
  '.jsx',
  '.lock',
  '.md',
  '.mjs',
  '.rs',
  '.sql',
  '.toml',
  '.ts',
  '.tsx',
  '.yaml',
  '.yml',
]);
const errors = [];

function normalizePath(value) {
  return value.replaceAll('\\', '/').replace(/\/+$/, '');
}

function hasGlob(value) {
  return value.includes('*');
}

function readExportManifest() {
  if (existsSync('tools/oss-export/manifest.json')) {
    return JSON.parse(readFileSync('tools/oss-export/manifest.json', 'utf8'));
  }

  return {
    include: [
      'apps',
      'libs',
      'deploy',
      'docs',
      'Cargo.toml',
      'package.json',
      'pnpm-lock.yaml',
      'pnpm-workspace.yaml',
    ],
    exclude: [
      'docs/cloud',
      'docs/internal',
      'docs/blueprint',
      'deploy/cloud',
      'deploy/internal',
      'libs/rust/adapters-cloud',
      'libs/rust/cloud',
      'libs/ts/cloud-ui',
      'apps/cloud-*',
      'apps/internal-*',
    ],
  };
}

const manifest = readExportManifest();

function isExcluded(path) {
  const normalized = normalizePath(path);
  return (manifest.exclude ?? []).some((entry) => {
    const exclude = normalizePath(entry);
    if (hasGlob(exclude)) {
      return normalized.startsWith(exclude.slice(0, exclude.indexOf('*')));
    }
    return normalized === exclude || normalized.startsWith(`${exclude}/`);
  });
}

function isTextFile(path) {
  if (path === 'Cargo.toml' || path === 'pnpm-lock.yaml' || path.endsWith('package.json')) {
    return true;
  }

  const dot = path.lastIndexOf('.');
  return dot >= 0 && textExtensions.has(path.slice(dot));
}

function providerTermsIn(content) {
  const lower = content.toLowerCase();
  return blockedTerms.filter((term) => lower.includes(term));
}

function scan(path, results) {
  if (!existsSync(path) || isExcluded(path)) return;

  const stat = lstatSync(path);
  if (stat.isSymbolicLink()) return;

  if (stat.isDirectory()) {
    for (const entry of readdirSync(path)) {
      if (skippedDirs.has(entry)) continue;
      scan(join(path, entry), results);
    }
    return;
  }

  const relativePath = normalizePath(relative(process.cwd(), path));
  if (policyFiles.has(relativePath) || !isTextFile(relativePath)) return;

  const terms = providerTermsIn(readFileSync(path, 'utf8'));
  if (terms.length > 0) {
    results.set(relativePath, terms);
  }
}

const findings = new Map();
for (const source of manifest.include ?? []) {
  scan(source, findings);
}

for (const path of knownProviderFiles) {
  if (forbiddenIdentityBillingBaselinePatterns.some((pattern) => pattern.test(path))) {
    errors.push(`${path}: Identity Billing runtime cannot remain an OSS provider baseline exception`);
  }
}

for (const [path, terms] of findings.entries()) {
  if (!knownProviderFiles.has(path)) {
    errors.push(`${path}: provider terms found (${terms.join(', ')})`);
  }
}

for (const path of knownProviderFiles) {
  if (!findings.has(path)) {
    errors.push(`${path}: stale provider baseline entry`);
  }
}

if (errors.length > 0) {
  console.error('OSS provider boundary violations:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  console.error('Move provider-specific code to adapters-cloud/cloud, or add a reviewed legacy exception.');
  process.exit(1);
}

console.log(`OSS provider boundary audit: ok (${knownProviderFiles.size} legacy files tracked)`);
