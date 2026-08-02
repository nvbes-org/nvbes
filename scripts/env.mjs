#!/usr/bin/env node

import { chmodSync, existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const workspaceRoot = resolve(fileURLToPath(new URL('..', import.meta.url)));
const templatePath = resolve(workspaceRoot, '.env.example');
const localPath = resolve(workspaceRoot, '.env');

function fail(message) {
  throw new Error(message);
}

function decodeValue(rawValue, path, lineNumber) {
  const value = rawValue.trim();
  if (value === '') return '';

  const quote = value[0];
  if (quote === "'") {
    const end = value.indexOf("'", 1);
    if (end === -1 || !/^\s*(?:#.*)?$/.test(value.slice(end + 1))) {
      fail(`${path}:${lineNumber}: invalid single-quoted value`);
    }
    return value.slice(1, end);
  }

  if (quote === '"') {
    let decoded = '';
    let escaped = false;
    let end = -1;
    for (let index = 1; index < value.length; index += 1) {
      const character = value[index];
      if (escaped) {
        decoded += { n: '\n', r: '\r', t: '\t' }[character] ?? character;
        escaped = false;
      } else if (character === '\\') {
        escaped = true;
      } else if (character === '"') {
        end = index;
        break;
      } else {
        decoded += character;
      }
    }
    if (end === -1 || !/^\s*(?:#.*)?$/.test(value.slice(end + 1))) {
      fail(`${path}:${lineNumber}: invalid double-quoted value`);
    }
    return decoded;
  }

  return value.replace(/\s+#.*$/, '').trimEnd();
}

export function parseEnv(content, path = '<env>', { allowDuplicates = false } = {}) {
  const assignments = new Map();
  const duplicates = new Set();
  const lines = content.replace(/\r\n/g, '\n').split('\n');

  for (const [index, line] of lines.entries()) {
    const trimmed = line.trim();
    if (trimmed === '' || trimmed.startsWith('#')) continue;

    const match = line.match(/^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/);
    if (!match) fail(`${path}:${index + 1}: expected KEY=value`);

    const [, key, rawValue] = match;
    if (assignments.has(key)) duplicates.add(key);
    assignments.set(key, {
      key,
      lineNumber: index + 1,
      rawValue,
      value: decodeValue(rawValue, path, index + 1),
    });
  }

  if (!allowDuplicates && duplicates.size > 0) {
    fail(
      `${path}: duplicate variables: ${[...duplicates].sort((left, right) => left.localeCompare(right)).join(', ')}`,
    );
  }

  return { assignments, duplicates, lines };
}

function shellQuote(value) {
  return `'${value.replaceAll("'", `'"'"'`)}'`;
}

function renderSynchronizedEnv(template, local, prune) {
  const rendered = template.lines.map((line) => {
    const match = line.match(/^(\s*(?:export\s+)?)([A-Za-z_][A-Za-z0-9_]*)(\s*=\s*)(.*)$/);
    if (!match) return line;
    const localAssignment = local.assignments.get(match[2]);
    return localAssignment ? `${match[1]}${match[2]}${match[3]}${localAssignment.rawValue}` : line;
  });

  const unknown = [...local.assignments.keys()]
    .filter((key) => !template.assignments.has(key))
    .sort((left, right) => left.localeCompare(right));
  if (!prune && unknown.length > 0) {
    rendered.push('', '# Local-only overrides (add them to .env.example if they are shared)');
    for (const key of unknown) rendered.push(`${key}=${local.assignments.get(key).rawValue}`);
  }

  return { content: `${rendered.join('\n').replace(/\n+$/, '')}\n`, unknown };
}

function load(path, options) {
  return parseEnv(readFileSync(path, 'utf8'), path, options);
}

function check() {
  const template = load(templatePath);
  if (!existsSync(localPath)) {
    console.log(
      `Environment contract: ${template.assignments.size} variables, local .env absent (ok)`,
    );
    return;
  }

  const local = load(localPath);
  const missing = [...template.assignments.keys()].filter((key) => !local.assignments.has(key));
  const unknown = [...local.assignments.keys()].filter((key) => !template.assignments.has(key));
  if (missing.length > 0 || unknown.length > 0) {
    const details = [
      missing.length > 0
        ? `missing: ${missing.sort((left, right) => left.localeCompare(right)).join(', ')}`
        : '',
      unknown.length > 0
        ? `unknown: ${unknown.sort((left, right) => left.localeCompare(right)).join(', ')}`
        : '',
    ].filter(Boolean);
    const resolution =
      unknown.length > 0
        ? 'Declare shared variables in .env.example or prune obsolete ones with `pnpm env:sync -- --prune`.'
        : 'Run `pnpm env:sync`.';
    fail(`.env is out of sync with .env.example (${details.join('; ')}). ${resolution}`);
  }

  console.log(`Environment contract: ${template.assignments.size} variables, local .env in sync`);
}

function sync({ prune = false } = {}) {
  const template = load(templatePath);
  const local = existsSync(localPath)
    ? load(localPath, { allowDuplicates: true })
    : { assignments: new Map(), duplicates: new Set(), lines: [] };
  const rendered = renderSynchronizedEnv(template, local, prune);

  writeFileSync(localPath, rendered.content, { encoding: 'utf8', mode: 0o600 });
  chmodSync(localPath, 0o600);

  const changes = [
    `${template.assignments.size} variables synchronized`,
    local.duplicates.size > 0 ? `${local.duplicates.size} duplicate(s) removed` : '',
    prune && rendered.unknown.length > 0
      ? `${rendered.unknown.length} obsolete variable(s) removed`
      : '',
    !prune && rendered.unknown.length > 0
      ? `${rendered.unknown.length} local-only variable(s) retained`
      : '',
  ].filter(Boolean);
  console.log(`Local .env: ${changes.join(', ')}`);
}

function exportMissing() {
  if (!existsSync(localPath)) return;
  const local = load(localPath);
  for (const { key, value } of local.assignments.values()) {
    if (process.env[key] === undefined) console.log(`export ${key}=${shellQuote(value)}`);
  }
}

function usage() {
  console.error('Usage: node scripts/env.mjs <check|sync|export> [--prune]');
  process.exitCode = 2;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const [command, ...args] = process.argv.slice(2);
  try {
    if (command === 'check') check();
    else if (command === 'sync') sync({ prune: args.includes('--prune') });
    else if (command === 'export') exportMissing();
    else usage();
  } catch (error) {
    console.error(`Environment error: ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  }
}
