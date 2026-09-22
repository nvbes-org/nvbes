#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const registryPath = 'docs/security/credential-stuffing-prevention-controls.json';
const errors = [];

function readText(path) {
  if (!existsSync(path)) {
    errors.push(`${path}: missing`);
    return '';
  }
  return readFileSync(path, 'utf8');
}

function readJson(path) {
  const text = readText(path);
  if (!text) return undefined;
  try {
    return JSON.parse(text);
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

function requireIncludes(path, includes, context) {
  const text = readText(path);
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

function assertRegistry(registry) {
  if (!registry) return;
  if (registry.schemaVersion !== 1) {
    errors.push(`${registryPath}: schemaVersion must be 1`);
  }
  requireString(registry.source, `${registryPath}.source`);
  requireString(registry.reviewCadence, `${registryPath}.reviewCadence`);

  const reqIds = new Set();
  const ctrlIds = new Set();
  let evidenceCount = 0;
  for (const [reqIndex, req] of requireArray(registry.requirements, 'requirements').entries()) {
    const reqPath = `${registryPath}.requirements[${reqIndex}]`;
    const reqId = requireString(req.id, `${reqPath}.id`);
    if (!/^CSPREV_REQ_\d{3}$/u.test(reqId)) errors.push(`${reqPath}.id: invalid ID ${reqId}`);
    if (reqIds.has(reqId)) errors.push(`${reqPath}.id: duplicate ${reqId}`);
    reqIds.add(reqId);
    requireString(req.name, `${reqPath}.name`);
    requireString(req.owasp, `${reqPath}.owasp`);
    for (const [ctrlIndex, ctrl] of requireArray(req.controls, `${reqPath}.controls`).entries()) {
      const ctrlPath = `${reqPath}.controls[${ctrlIndex}]`;
      const ctrlId = requireString(ctrl.id, `${ctrlPath}.id`);
      if (!/^CSPREV_CTRL_\d{3}$/u.test(ctrlId)) errors.push(`${ctrlPath}.id: invalid ID ${ctrlId}`);
      if (ctrlIds.has(ctrlId)) errors.push(`${ctrlPath}.id: duplicate ${ctrlId}`);
      ctrlIds.add(ctrlId);
      requireString(ctrl.name, `${ctrlPath}.name`);
      requireString(ctrl.description, `${ctrlPath}.description`);
      for (const [evidenceIndex, evidence] of requireArray(
        ctrl.evidence,
        `${ctrlPath}.evidence`,
      ).entries()) {
        evidenceCount += requireIncludes(
          requireString(evidence.path, `${ctrlPath}.evidence[${evidenceIndex}].path`),
          evidence.includes,
          `${ctrlPath}.evidence[${evidenceIndex}]`,
        );
      }
    }
  }
  if (reqIds.size < 8) errors.push(`${registryPath}: expected at least 8 requirements`);
  if (ctrlIds.size < 8) errors.push(`${registryPath}: expected at least 8 controls`);
  if (evidenceCount < 16) errors.push(`${registryPath}: expected substantial evidence coverage`);
}

function assertImplementation() {
  const limiter = readText('libs/rust/core/src/limiter.rs');
  for (const needle of [
    'RateLimiter',
    'RateLimitRule',
    'check_rate_limit',
    'max_hits',
    'rate_limited',
  ]) {
    if (!limiter.includes(needle)) errors.push(`limiter.rs missing ${needle}`);
  }
}

function assertLayeredDefenses() {
  const auth = readText('apps/identity-service/src/identity.auth.rs');
  const password = readText('libs/rust/core/src/auth.helpers.password.rs');
  const validation = readText('libs/rust/core/src/auth.helpers.validation.rs');
  const assessment = readText('libs/rust/trust-risk/src/trust_risk.assessment.rs');

  for (const needle of ['dummy_verify_password', 'normalize_email', 'authentication failed']) {
    if (!auth.includes(needle)) errors.push(`identity.auth.rs missing ${needle}`);
  }
  for (const needle of ['hash_password', 'verify_password', 'argon2']) {
    if (!password.includes(needle)) errors.push(`auth.helpers.password.rs missing ${needle}`);
  }
  for (const needle of ['validate_password', 'MIN_PASSWORD_LENGTH', 'MAX_PASSWORD_LENGTH']) {
    if (!validation.includes(needle)) errors.push(`auth.helpers.validation.rs missing ${needle}`);
  }
  for (const needle of ['Assessment', 'instantaneous_signals', 'RiskSignal']) {
    if (!assessment.includes(needle)) errors.push(`trust_risk.assessment.rs missing ${needle}`);
  }
}

function assertPackageScript() {
  const pkg = readText('package.json');
  if (!pkg.includes('check:credential-stuffing-prevention')) {
    errors.push('package.json: check:credential-stuffing-prevention script missing');
  }
  if (!pkg.includes('tools/security/check-credential-stuffing-prevention.mjs')) {
    errors.push('package.json: credential stuffing checker command missing');
  }
}

assertRegistry(readJson(registryPath));
assertImplementation();
assertLayeredDefenses();
assertPackageScript();

if (errors.length > 0) {
  console.error('Credential Stuffing Prevention controls failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('Credential Stuffing Prevention controls OK');
