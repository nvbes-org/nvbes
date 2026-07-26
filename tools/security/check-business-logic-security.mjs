#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const registryPath = 'docs/security/business-logic-security-controls.json';
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

const registry = readJson(registryPath);

if (registry) {
  if (registry.schemaVersion !== expectedSchemaVersion) {
    errors.push(`${registryPath}: schemaVersion must be ${expectedSchemaVersion}`);
  }

  requireString(registry.source, `${registryPath}.source`);
  requireString(registry.reviewCadence, `${registryPath}.reviewCadence`);

  const requirements = requireArray(registry.requirements, `${registryPath}.requirements`);
  const requirementIds = new Set();
  const controlIds = new Set();
  let controlCount = 0;
  let evidenceCount = 0;

  requirements.forEach((requirement, requirementIndex) => {
    const requirementContext = `requirements[${requirementIndex}]`;
    const requirementId = assertId(requirement?.id, `${requirementContext}.id`, /^BLS_REQ_\d{3}$/u);
    requireUnique(requirementId, requirementIds, 'requirements');
    requireString(requirement?.name, `${requirementContext}.name`);
    requireString(requirement?.owasp, `${requirementContext}.owasp`);

    const controls = requireArray(requirement?.controls, `${requirementContext}.controls`);
    if (controls.length === 0) {
      errors.push(`${requirementContext}: must include at least one control`);
    }

    controls.forEach((control, controlIndex) => {
      const controlContext = `${requirementContext}.controls[${controlIndex}]`;
      const controlId = assertId(control?.id, `${controlContext}.id`, /^BLS_CTRL_\d{3}$/u);
      requireUnique(controlId, controlIds, 'controls');
      requireString(control?.name, `${controlContext}.name`);
      requireString(control?.description, `${controlContext}.description`);
      controlCount += 1;

      const evidence = requireArray(control?.evidence, `${controlContext}.evidence`);
      if (evidence.length === 0) {
        errors.push(`${controlContext}: must include at least one evidence entry`);
      }

      evidence.forEach((entry, evidenceIndex) => {
        const evidenceContext = `${controlContext}.evidence[${evidenceIndex}]`;
        const path = requireString(entry?.path, `${evidenceContext}.path`);
        evidenceCount += requireIncludes(path, entry?.includes, evidenceContext);
      });
    });
  });

  if (errors.length === 0) {
    console.log(
      `Business logic security controls: ok (${requirements.length} requirements, ${controlCount} controls, ${evidenceCount} evidence strings)`,
    );
  }
}

if (errors.length > 0) {
  console.error('Business logic security controls failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}
