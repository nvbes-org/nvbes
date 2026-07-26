#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const registryPath = 'docs/security/authorization-regression-tests.json';
const packageJsonPath = 'package.json';
const expectedSchemaVersion = 1;
const errors = [];

function readJson(path) {
  if (!existsSync(path)) {
    errors.push(`${path}: missing`);
    return undefined;
  }

  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    errors.push(`${path}: invalid JSON (${error.message})`);
    return undefined;
  }
}

function requireArray(value, path) {
  if (Array.isArray(value)) return value;
  errors.push(`${path}: must be an array`);
  return [];
}

function requireString(value, path) {
  if (typeof value === 'string' && value.trim().length > 0) return value;
  errors.push(`${path}: must be a non-empty string`);
  return '';
}

function assertId(value, path, pattern) {
  const id = requireString(value, path);
  if (id && !pattern.test(id)) {
    errors.push(`${path}: invalid ID format (${id})`);
  }
  return id;
}

function requireUnique(id, seen, path) {
  if (!id) return;
  if (seen.has(id)) {
    errors.push(`${path}: duplicate ID ${id}`);
    return;
  }
  seen.add(id);
}

function requireIncludes(path, includes, context) {
  if (!existsSync(path)) {
    errors.push(`${context}.path: ${path} is missing`);
    return 0;
  }

  const text = readFileSync(path, 'utf8');
  let count = 0;

  for (const include of requireArray(includes, `${context}.includes`)) {
    const needle = requireString(include, `${context}.includes[]`);
    if (!needle) continue;
    count += 1;
    if (!text.includes(needle)) {
      errors.push(`${context}: ${path} does not include ${JSON.stringify(needle)}`);
    }
  }

  return count;
}

function validateEvidence(entries, context) {
  const evidence = requireArray(entries, `${context}.evidence`);
  if (evidence.length === 0) {
    errors.push(`${context}: must include at least one evidence entry`);
  }

  let evidenceCount = 0;
  evidence.forEach((entry, evidenceIndex) => {
    const evidenceContext = `${context}.evidence[${evidenceIndex}]`;
    const path = requireString(entry?.path, `${evidenceContext}.path`);
    evidenceCount += requireIncludes(path, entry?.includes, evidenceContext);
  });
  return evidenceCount;
}

function validatePnpmScript(command, scripts, context) {
  const prefix = 'pnpm ';
  if (!command.startsWith(prefix)) return;

  const scriptName = command.slice(prefix.length).trim().split(/\s+/u)[0];
  if (!scriptName.startsWith('check:')) return;

  if (!Object.hasOwn(scripts, scriptName)) {
    errors.push(`${context}.command: package.json is missing script ${scriptName}`);
  }
}

const registry = readJson(registryPath);
const packageJson = readJson(packageJsonPath);

if (registry) {
  if (registry.schemaVersion !== expectedSchemaVersion) {
    errors.push(`${registryPath}: schemaVersion must be ${expectedSchemaVersion}`);
  }

  requireString(registry.source, `${registryPath}.source`);
  requireString(registry.reviewCadence, `${registryPath}.reviewCadence`);

  const scripts =
    packageJson?.scripts && typeof packageJson.scripts === 'object' ? packageJson.scripts : {};
  const commands = requireArray(registry.commands, `${registryPath}.commands`);
  const suites = requireArray(registry.suites, `${registryPath}.suites`);
  const commandIds = new Set();
  const suiteIds = new Set();
  let evidenceCount = 0;

  commands.forEach((commandEntry, commandIndex) => {
    const commandContext = `commands[${commandIndex}]`;
    const commandId = assertId(commandEntry?.id, `${commandContext}.id`, /^AUTHZ_REG_CMD_\d{3}$/u);
    requireUnique(commandId, commandIds, 'commands');
    requireString(commandEntry?.name, `${commandContext}.name`);
    const command = requireString(commandEntry?.command, `${commandContext}.command`);
    requireString(commandEntry?.purpose, `${commandContext}.purpose`);
    validatePnpmScript(command, scripts, commandContext);
    evidenceCount += validateEvidence(commandEntry?.evidence, commandContext);
  });

  suites.forEach((suite, suiteIndex) => {
    const suiteContext = `suites[${suiteIndex}]`;
    const suiteId = assertId(suite?.id, `${suiteContext}.id`, /^AUTHZ_REG_\d{3}$/u);
    requireUnique(suiteId, suiteIds, 'suites');
    requireString(suite?.name, `${suiteContext}.name`);
    requireString(suite?.owasp, `${suiteContext}.owasp`);
    requireString(suite?.pattern, `${suiteContext}.pattern`);
    requireString(suite?.regressionRisk, `${suiteContext}.regressionRisk`);
    evidenceCount += validateEvidence(suite?.evidence, suiteContext);
  });

  if (errors.length === 0) {
    console.log(
      `Authorization regression tests: ok (${commands.length} commands, ${suites.length} suites, ${evidenceCount} evidence strings)`,
    );
  }
}

if (errors.length > 0) {
  console.error('Authorization regression tests failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}
