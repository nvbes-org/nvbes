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

  const hashIndex = value.indexOf(' #');
  const withoutComment = hashIndex >= 0 ? value.slice(0, hashIndex) : value;
  return withoutComment.trimEnd();
}

function isHorizontalSpace(character) {
  return character === ' ' || character === '\t';
}

function isAsciiLetter(character) {
  return (character >= 'a' && character <= 'z') || (character >= 'A' && character <= 'Z');
}

function isIdentifierPart(character) {
  return isAsciiLetter(character) || (character >= '0' && character <= '9') || character === '_';
}

function readIdentifier(line, start, limit) {
  const first = line[start];
  if (start >= limit || first === undefined || !(first === '_' || isAsciiLetter(first))) {
    return -1;
  }
  let index = start + 1;
  while (index < limit && isIdentifierPart(line[index])) index += 1;
  return index;
}

/**
 * Candidate offsets where the key of an assignment can start. The optional
 * `export` prefix is preferred, but a key literally named `export` stays
 * reachable, mirroring the greedy-with-backtracking behaviour it replaces.
 */
function keyStartCandidates(line, indentationEnd) {
  const candidates = [indentationEnd];
  if (!line.startsWith('export', indentationEnd)) return candidates;

  let afterExport = indentationEnd + 'export'.length;
  if (!isHorizontalSpace(line[afterExport])) return candidates;
  while (isHorizontalSpace(line[afterExport])) afterExport += 1;
  candidates.unshift(afterExport);
  return candidates;
}

/**
 * Parse a `KEY=value` line, tolerating an optional `export` prefix and spaces
 * around the assignment operator. Returns null when the line is not an
 * assignment so the caller can fail with a precise diagnostic.
 */
function splitAssignment(line) {
  const equalsIndex = line.indexOf('=');
  if (equalsIndex === -1) return null;

  for (const keyStart of keyStartCandidates(line, 0)) {
    const keyEnd = readIdentifier(line, keyStart, equalsIndex);
    if (keyEnd === -1) continue;

    let operatorIndex = keyEnd;
    while (operatorIndex < equalsIndex && isHorizontalSpace(line[operatorIndex]))
      operatorIndex += 1;
    if (operatorIndex !== equalsIndex) continue;

    let valueStart = equalsIndex + 1;
    while (valueStart < line.length && isHorizontalSpace(line[valueStart])) valueStart += 1;

    return { key: line.slice(keyStart, keyEnd), rawValue: line.slice(valueStart) };
  }

  return null;
}

/**
 * Split a template line into its indentation (with an optional `export`), its
 * key and the operator/whitespace separator so the value can be replaced while
 * the original formatting is preserved.
 */
export function splitTemplateAssignment(line) {
  let indentationEnd = 0;
  while (indentationEnd < line.length && isHorizontalSpace(line[indentationEnd])) {
    indentationEnd += 1;
  }

  for (const prefixEnd of keyStartCandidates(line, indentationEnd)) {
    const keyEnd = readIdentifier(line, prefixEnd, line.length);
    if (keyEnd === -1) continue;

    let equalsIndex = keyEnd;
    while (isHorizontalSpace(line[equalsIndex])) equalsIndex += 1;
    if (line[equalsIndex] !== '=') continue;

    let valueStart = equalsIndex + 1;
    while (isHorizontalSpace(line[valueStart])) valueStart += 1;

    return {
      key: line.slice(prefixEnd, keyEnd),
      prefix: line.slice(0, prefixEnd),
      separator: line.slice(keyEnd, valueStart),
    };
  }

  return null;
}

export function parseEnv(content, path = '<env>', { allowDuplicates = false } = {}) {
  const assignments = new Map();
  const duplicates = new Set();
  const lines = content.replace(/\r\n/g, '\n').split('\n');

  for (const [index, line] of lines.entries()) {
    const trimmed = line.trim();
    if (trimmed === '' || trimmed.startsWith('#')) continue;

    const assignment = splitAssignment(trimmed);
    if (!assignment) fail(`${path}:${index + 1}: expected KEY=value`);

    const { key, rawValue } = assignment;
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
    const parts = splitTemplateAssignment(line);
    if (!parts) return line;
    const localAssignment = local.assignments.get(parts.key);
    return localAssignment
      ? `${parts.prefix}${parts.key}${parts.separator}${localAssignment.rawValue}`
      : line;
  });

  const unknown = [...local.assignments.keys()]
    .filter((key) => !template.assignments.has(key))
    .sort((left, right) => left.localeCompare(right));
  if (!prune && unknown.length > 0) {
    rendered.push('', '# Local-only overrides (add them to .env.example if they are shared)');
    for (const key of unknown) rendered.push(`${key}=${local.assignments.get(key).rawValue}`);
  }

  let text = rendered.join('\n');
  while (text.endsWith('\n')) text = text.slice(0, -1);
  return { content: `${text}\n`, unknown };
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
