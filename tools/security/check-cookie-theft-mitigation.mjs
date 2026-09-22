#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const registryPath = 'docs/security/cookie-theft-mitigation-controls.json';
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
    if (!/^CTM_REQ_\d{3}$/u.test(reqId)) errors.push(`${reqPath}.id: invalid ID ${reqId}`);
    if (reqIds.has(reqId)) errors.push(`${reqPath}.id: duplicate ${reqId}`);
    reqIds.add(reqId);
    requireString(req.name, `${reqPath}.name`);
    requireString(req.owasp, `${reqPath}.owasp`);
    for (const [ctrlIndex, ctrl] of requireArray(req.controls, `${reqPath}.controls`).entries()) {
      const ctrlPath = `${reqPath}.controls[${ctrlIndex}]`;
      const ctrlId = requireString(ctrl.id, `${ctrlPath}.id`);
      if (!/^CTM_CTRL_\d{3}$/u.test(ctrlId)) errors.push(`${ctrlPath}.id: invalid ID ${ctrlId}`);
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
  const auth = readText('apps/identity-service/src/identity.auth.rs');
  for (const field of ['identity_sessions', 'hash_token', 'expires_at', 'SESSION_TTL_HOURS']) {
    if (!auth.includes(field)) errors.push(`identity.auth.rs missing ${field}`);
  }

  const headers = readText('libs/rust/core/src/security.headers.rs');
  for (const needle of ['ACCEPT_CH_VALUE', 'CRITICAL_CH_VALUE', 'CLEAR_SITE_DATA_VALUE']) {
    if (!headers.includes(needle)) errors.push(`security.headers.rs missing ${needle}`);
  }
}

function assertRequestAwareAuth() {
  const http = readText('apps/identity-service/src/identity.http.rs');
  const tokens = readText('apps/identity-service/src/identity.tokens.rs');
  /** @type {Array<[string, string, string[]]>} */
  const requestAwareAuthChecks = [
    ['login routes', http, ['LoginRequest', 'LoginResponse', 'session_token']],
    ['session tokens', tokens, ['AccessTokenClaims', 'TokenService', 'pub fn verify(']],
  ];
  for (const [path, text, needles] of requestAwareAuthChecks) {
    for (const needle of needles)
      if (!text.includes(needle)) errors.push(`${path}: missing ${needle}`);
  }
}

function assertReauthenticationAndTelemetry() {
  const httpError = readText('libs/rust/core/src/http.error.rs');
  for (const needle of [
    'reauthentication_error_explicitly_requests_reauthentication',
    'requiring_reauthentication',
    'ErrorRecovery::Reauthenticate',
  ]) {
    if (!httpError.includes(needle)) errors.push(`http.error.rs missing ${needle}`);
  }

  const auth = readText('apps/identity-service/src/identity.auth.rs');
  for (const needle of [
    'identity.authenticated',
    'audit(&mut tx, principal_id, "identity.authenticated")',
  ]) {
    if (!auth.includes(needle)) errors.push(`identity.auth.rs missing ${needle}`);
  }
}

function assertCookieAttributes() {
  const headers = readText('libs/rust/core/src/security.headers.rs');
  for (const needle of [
    'CLEAR_SITE_DATA_VALUE',
    'insert_clear_site_data_header',
    'CSP_VALUE',
    "frame-ancestors 'none'",
  ]) {
    if (!headers.includes(needle)) errors.push(`security headers missing ${needle}`);
  }
}

function assertPackageScript() {
  const pkg = readText('package.json');
  if (!pkg.includes('check:cookie-theft-mitigation')) {
    errors.push('package.json: check:cookie-theft-mitigation script missing');
  }
  if (!pkg.includes('tools/security/check-cookie-theft-mitigation.mjs')) {
    errors.push('package.json: cookie theft checker command missing');
  }
}

assertRegistry(readJson(registryPath));
assertImplementation();
assertRequestAwareAuth();
assertReauthenticationAndTelemetry();
assertCookieAttributes();
assertPackageScript();

if (errors.length > 0) {
  console.error('Cookie Theft Mitigation controls failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('Cookie Theft Mitigation controls OK');
