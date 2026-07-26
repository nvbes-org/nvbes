#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const matrixPath = 'docs/security/authorization-testing-automation-matrix.json';
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

function requireObject(value, path) {
  if (value && typeof value === 'object' && !Array.isArray(value)) return value;
  errors.push(`${path}: must be an object`);
  return {};
}

function requireString(value, path) {
  if (typeof value === 'string' && value.trim().length > 0) return value;
  errors.push(`${path}: must be a non-empty string`);
  return '';
}

function requireStatus(value, path, allowedStatuses) {
  if (Number.isInteger(value) && allowedStatuses.has(value)) return value;
  errors.push(`${path}: must be one of ${Array.from(allowedStatuses).join(', ')}`);
  return 0;
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

function validateRoleCoverage(service, pointOfViewIds, context) {
  const allowed = new Set(requireArray(service?.allowed, `${context}.allowed`));
  const denied = new Set(requireArray(service?.denied, `${context}.denied`));

  for (const id of [...allowed, ...denied]) {
    if (!pointOfViewIds.has(id)) {
      errors.push(`${context}: unknown point of view ${id}`);
    }
  }

  for (const id of pointOfViewIds) {
    const isAllowed = allowed.has(id);
    const isDenied = denied.has(id);
    if (isAllowed && isDenied) {
      errors.push(`${context}: point of view ${id} is both allowed and denied`);
    } else if (!isAllowed && !isDenied) {
      errors.push(`${context}: point of view ${id} is not classified`);
    }
  }

  return allowed.size + denied.size;
}

const matrix = readJson(matrixPath);
const packageJson = readJson(packageJsonPath);

if (matrix) {
  if (matrix.schemaVersion !== expectedSchemaVersion) {
    errors.push(`${matrixPath}: schemaVersion must be ${expectedSchemaVersion}`);
  }

  requireString(matrix.source, `${matrixPath}.source`);
  requireString(matrix.reviewCadence, `${matrixPath}.reviewCadence`);

  const statusCodes = requireObject(matrix.statusCodes, `${matrixPath}.statusCodes`);
  const allowedStatuses = new Set(
    requireArray(statusCodes.allowed, `${matrixPath}.statusCodes.allowed`),
  );
  const deniedStatuses = new Set(
    requireArray(statusCodes.denied, `${matrixPath}.statusCodes.denied`),
  );
  const pointsOfView = requireArray(matrix.pointsOfView, `${matrixPath}.pointsOfView`);
  const services = requireArray(matrix.services, `${matrixPath}.services`);
  const servicesTesting = requireArray(matrix.servicesTesting, `${matrixPath}.servicesTesting`);
  const pointOfViewIds = new Set();
  const serviceIds = new Set();
  const serviceTestingIds = new Set();
  let evidenceCount = 0;
  let caseCount = 0;

  pointsOfView.forEach((pointOfView, pointOfViewIndex) => {
    const context = `pointsOfView[${pointOfViewIndex}]`;
    const id = assertId(pointOfView?.id, `${context}.id`, /^[a-z][a-z0-9_]*$/u);
    requireUnique(id, pointOfViewIds, 'pointsOfView');
    requireString(pointOfView?.role, `${context}.role`);
    requireString(pointOfView?.authContext, `${context}.authContext`);
    requireString(pointOfView?.description, `${context}.description`);
  });

  servicesTesting.forEach((fixture, fixtureIndex) => {
    const context = `servicesTesting[${fixtureIndex}]`;
    const serviceId = assertId(
      fixture?.serviceId,
      `${context}.serviceId`,
      /^AUTHZ_AUTO_SVC_\d{3}$/u,
    );
    requireUnique(serviceId, serviceTestingIds, 'servicesTesting');
    requireObject(fixture?.pathParameters, `${context}.pathParameters`);
    if (!Object.hasOwn(fixture, 'payload')) {
      errors.push(`${context}.payload: must be present, use null when the service has no body`);
    }
  });

  services.forEach((service, serviceIndex) => {
    const context = `services[${serviceIndex}]`;
    const id = assertId(service?.id, `${context}.id`, /^AUTHZ_AUTO_SVC_\d{3}$/u);
    requireUnique(id, serviceIds, 'services');
    requireString(service?.name, `${context}.name`);
    requireString(service?.resource, `${context}.resource`);
    requireString(service?.action, `${context}.action`);
    requireString(service?.method, `${context}.method`);
    requireString(service?.uri, `${context}.uri`);
    requireStatus(
      service?.expectedAllowedStatus,
      `${context}.expectedAllowedStatus`,
      allowedStatuses,
    );
    requireStatus(service?.expectedDeniedStatus, `${context}.expectedDeniedStatus`, deniedStatuses);
    caseCount += validateRoleCoverage(service, pointOfViewIds, context);
    evidenceCount += validateEvidence(service?.evidence, context);

    if (!serviceTestingIds.has(id)) {
      errors.push(`${context}: missing servicesTesting fixture for ${id}`);
    }
  });

  for (const fixtureId of serviceTestingIds) {
    if (!serviceIds.has(fixtureId)) {
      errors.push(`servicesTesting: fixture references unknown service ${fixtureId}`);
    }
  }

  const scripts =
    packageJson?.scripts && typeof packageJson.scripts === 'object' ? packageJson.scripts : {};
  if (!Object.hasOwn(scripts, 'check:authorization-testing-automation')) {
    errors.push('package.json: missing check:authorization-testing-automation script');
  }
  if (
    typeof scripts.check === 'string' &&
    !scripts.check.includes('pnpm check:authorization-testing-automation')
  ) {
    errors.push(
      'package.json: root check must include pnpm check:authorization-testing-automation',
    );
  }

  if (errors.length === 0) {
    console.log(
      `Authorization testing automation: ok (${pointsOfView.length} points of view, ${services.length} services, ${caseCount} role/service cases, ${evidenceCount} evidence strings)`,
    );
  }
}

if (errors.length > 0) {
  console.error('Authorization testing automation failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}
