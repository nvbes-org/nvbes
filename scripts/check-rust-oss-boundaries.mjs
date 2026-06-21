#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { relative } from 'node:path';

function run(command, args) {
  return execFileSync(command, args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

const metadata = JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps']));
const workspaceIds = new Set(metadata.workspace_members);
const workspacePackages = metadata.packages.filter((pkg) => workspaceIds.has(pkg.id));
const packagesByName = new Map(workspacePackages.map((pkg) => [pkg.name, pkg]));

function scopeForPackage(pkg) {
  const manifestPath = relative(process.cwd(), pkg.manifest_path);

  if (
    manifestPath.startsWith('apps/internal-') ||
    manifestPath.startsWith('libs/rust/internal') ||
    manifestPath.includes('/internal/')
  ) {
    return 'internal';
  }

  if (
    manifestPath.startsWith('apps/cloud-') ||
    manifestPath.startsWith('libs/rust/cloud/') ||
    manifestPath.startsWith('libs/rust/adapters-cloud/')
  ) {
    return 'cloud';
  }

  return 'oss';
}

const scopeByPackage = new Map(workspacePackages.map((pkg) => [pkg.name, scopeForPackage(pkg)]));
const directDeps = new Map();

for (const pkg of workspacePackages) {
  directDeps.set(
    pkg.name,
    pkg.dependencies
      .map((dep) => packagesByName.get(dep.name))
      .filter(Boolean)
      .map((depPkg) => depPkg.name),
  );
}

function transitiveWorkspaceDeps(pkgName, seen = new Set()) {
  const deps = directDeps.get(pkgName) ?? [];
  for (const depName of deps) {
    if (seen.has(depName)) continue;
    seen.add(depName);
    transitiveWorkspaceDeps(depName, seen);
  }
  return seen;
}

const errors = [];

for (const pkg of workspacePackages) {
  const sourceScope = scopeByPackage.get(pkg.name);
  const deps = transitiveWorkspaceDeps(pkg.name);

  for (const depName of deps) {
    const targetScope = scopeByPackage.get(depName);

    if (sourceScope === 'oss' && ['cloud', 'internal'].includes(targetScope)) {
      errors.push(`${pkg.name} cannot depend on ${targetScope} crate ${depName}`);
    }

    if (sourceScope === 'cloud' && targetScope === 'internal') {
      errors.push(`${pkg.name} cannot depend on internal crate ${depName}`);
    }
  }
}

if (errors.length > 0) {
  console.error('Rust OSS dependency boundary violations:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log('Rust OSS dependency boundaries: ok');
