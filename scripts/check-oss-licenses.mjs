#!/usr/bin/env node
import { existsSync, lstatSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const allowedLicenses = new Set([
  'AGPL-3.0-only',
  'Apache-2.0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'ISC',
  'MIT',
  'MPL-2.0',
  '0BSD',
]);

const errors = [];

function walkDirs(root, predicate) {
  if (!existsSync(root)) return [];
  const matches = [];
  const stack = [root];

  while (stack.length > 0) {
    const current = stack.pop();
    const currentStat = lstatSync(current);
    if (!currentStat.isDirectory() || currentStat.isSymbolicLink()) continue;
    if (predicate(current)) matches.push(current);

    for (const entry of readdirSync(current)) {
      if (['.git', '.nx', 'coverage', 'dist', 'node_modules', 'target'].includes(entry)) continue;
      const next = join(current, entry);
      const nextStat = lstatSync(next);
      if (nextStat.isDirectory() && !nextStat.isSymbolicLink()) stack.push(next);
    }
  }

  return matches;
}

function exportIncludes() {
  if (existsSync('tools/oss-export/manifest.json')) {
    return JSON.parse(readFileSync('tools/oss-export/manifest.json', 'utf8')).include;
  }

  return [
    ...walkDirs('apps', (dir) => existsSync(join(dir, 'Cargo.toml')) || existsSync(join(dir, 'package.json'))),
    ...walkDirs('libs/rust', (dir) => existsSync(join(dir, 'Cargo.toml'))),
    ...walkDirs('libs/ts', (dir) => existsSync(join(dir, 'package.json'))),
  ];
}

function parseTomlLicense(content) {
  const explicit = content.match(/^license\s*=\s*"([^"]+)"/m)?.[1];
  if (explicit) return explicit;
  if (/^license\.workspace\s*=\s*true/m.test(content)) {
    return parseTomlLicense(readFileSync('Cargo.toml', 'utf8'));
  }
  return null;
}

function checkLicense(label, license) {
  if (!license) {
    errors.push(`${label}: missing license`);
    return;
  }

  if (!allowedLicenses.has(license)) {
    errors.push(`${label}: unsupported OSS license ${license}`);
  }
}

checkLicense('LICENSE policy', readFileSync('LICENSE', 'utf8').match(/SPDX-License-Identifier:\s*([^\s]+)/)?.[1]);
const rootPackagePath = existsSync('package.oss.json') ? 'package.oss.json' : 'package.json';
checkLicense(rootPackagePath, JSON.parse(readFileSync(rootPackagePath, 'utf8')).license);
checkLicense('Cargo.toml workspace', parseTomlLicense(readFileSync('Cargo.toml', 'utf8')));

for (const path of exportIncludes()) {
  if (path.startsWith('libs/ts/')) {
    const packagePath = join(path, 'package.json');
    const pkg = JSON.parse(readFileSync(packagePath, 'utf8'));
    checkLicense(packagePath, pkg.license);
  }

  if (path.startsWith('libs/rust/') || path.startsWith('apps/')) {
    try {
      const cargoPath = join(path, 'Cargo.toml');
      const license = parseTomlLicense(readFileSync(cargoPath, 'utf8'));
      checkLicense(cargoPath, license);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }
}

if (errors.length > 0) {
  console.error('OSS license policy violations:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log('OSS license policy: ok');
