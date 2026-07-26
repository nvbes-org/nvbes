#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const registryPath = 'docs/security/abuse-cases.json';
const expectedSchemaVersion = 1;
const validRisks = new Set(['critical', 'high', 'medium', 'low']);
const validKinds = new Set(['technical', 'business']);
const validDecisions = new Set(['to_address', 'accepted']);
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

function indexById(items, path, pattern) {
  const map = new Map();

  items.forEach((item, index) => {
    const id = assertId(item?.id, `${path}[${index}].id`, pattern);
    if (!id) return;
    if (map.has(id)) {
      errors.push(`${path}: duplicate ID ${id}`);
      return;
    }
    map.set(id, item);
  });

  return map;
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

  const features = requireArray(registry.features, `${registryPath}.features`);
  const countermeasures = requireArray(registry.countermeasures, `${registryPath}.countermeasures`);
  const abuseCases = requireArray(registry.abuseCases, `${registryPath}.abuseCases`);

  const featureIds = indexById(features, 'features', /^FEATURE_\d{3}$/u);
  const countermeasureIds = indexById(countermeasures, 'countermeasures', /^DEFENSE_\d{3}$/u);
  const abuseCaseIds = indexById(abuseCases, 'abuseCases', /^ABUSE_CASE_\d{3}$/u);

  let validationCount = 0;

  for (const [id, abuseCase] of abuseCaseIds) {
    const context = `abuseCases.${id}`;
    const featureId = assertId(abuseCase.featureId, `${context}.featureId`, /^FEATURE_\d{3}$/u);
    if (featureId && !featureIds.has(featureId)) {
      errors.push(`${context}.featureId: unknown feature ${featureId}`);
    }

    const kind = requireString(abuseCase.kind, `${context}.kind`);
    if (kind && !validKinds.has(kind)) errors.push(`${context}.kind: invalid kind ${kind}`);

    const risk = requireString(abuseCase.risk, `${context}.risk`);
    if (risk && !validRisks.has(risk)) errors.push(`${context}.risk: invalid risk ${risk}`);

    const decision = requireString(abuseCase.decision, `${context}.decision`);
    if (decision && !validDecisions.has(decision)) {
      errors.push(`${context}.decision: invalid decision ${decision}`);
    }

    requireString(abuseCase.attack, `${context}.attack`);
    requireString(abuseCase.referential, `${context}.referential`);

    const linkedCountermeasures = requireArray(
      abuseCase.countermeasures,
      `${context}.countermeasures`,
    );
    for (const countermeasureId of linkedCountermeasures) {
      const value = assertId(countermeasureId, `${context}.countermeasures[]`, /^DEFENSE_\d{3}$/u);
      if (value && !countermeasureIds.has(value)) {
        errors.push(`${context}.countermeasures: unknown countermeasure ${value}`);
      }
    }

    const validations = requireArray(abuseCase.validation, `${context}.validation`);
    for (const [index, validation] of validations.entries()) {
      const validationContext = `${context}.validation[${index}]`;
      const path = requireString(validation?.path, `${validationContext}.path`);
      validationCount += requireIncludes(path, validation?.includes, validationContext);
    }

    if (decision === 'to_address') {
      if (linkedCountermeasures.length === 0) {
        errors.push(`${context}: addressed abuse cases must reference at least one countermeasure`);
      }
      if (validations.length === 0) {
        errors.push(`${context}: addressed abuse cases must provide validation evidence`);
      }
    }

    if (decision === 'accepted') {
      requireString(abuseCase.acceptance, `${context}.acceptance`);
    }
  }

  if (errors.length === 0) {
    console.log(
      `Abuse case registry: ok (${abuseCaseIds.size} abuse cases, ${validationCount} evidence strings)`,
    );
  }
}

if (errors.length > 0) {
  console.error('Abuse case registry failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}
