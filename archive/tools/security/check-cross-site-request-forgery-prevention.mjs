#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const registryPath = 'docs/security/cross-site-request-forgery-prevention-controls.json';
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
  if (typeof value === 'string' && value.trim()) return value;
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
  if (registry.schemaVersion !== 1) errors.push(`${registryPath}: schemaVersion must be 1`);
  requireString(registry.source, `${registryPath}.source`);
  requireString(registry.reviewCadence, `${registryPath}.reviewCadence`);

  const reqIds = new Set();
  const ctrlIds = new Set();
  let evidenceCount = 0;
  for (const [reqIndex, req] of requireArray(registry.requirements, 'requirements').entries()) {
    const reqPath = `${registryPath}.requirements[${reqIndex}]`;
    const reqId = requireString(req.id, `${reqPath}.id`);
    if (!/^CSRF_REQ_\d{3}$/u.test(reqId)) errors.push(`${reqPath}.id: invalid ID ${reqId}`);
    if (reqIds.has(reqId)) errors.push(`${reqPath}.id: duplicate ${reqId}`);
    reqIds.add(reqId);
    requireString(req.name, `${reqPath}.name`);
    requireString(req.owasp, `${reqPath}.owasp`);
    for (const [ctrlIndex, ctrl] of requireArray(req.controls, `${reqPath}.controls`).entries()) {
      const ctrlPath = `${reqPath}.controls[${ctrlIndex}]`;
      const ctrlId = requireString(ctrl.id, `${ctrlPath}.id`);
      if (!/^CSRF_CTRL_\d{3}$/u.test(ctrlId)) {
        errors.push(`${ctrlPath}.id: invalid ID ${ctrlId}`);
      }
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
  if (reqIds.size < 9) errors.push(`${registryPath}: expected at least 9 requirements`);
  if (ctrlIds.size < 9) errors.push(`${registryPath}: expected at least 9 controls`);
  if (evidenceCount < 58) errors.push(`${registryPath}: expected substantial evidence coverage`);
}

function assertImplementation() {
  const cookies = readText('apps/identity-service/src/identity.http.cookies.rs');
  for (const needle of [
    'generate_csrf_token(session_token: &str, secret: &str)',
    'verify_csrf_token(csrf_token: &str, session_token: &str, secret: &str)',
    'HmacSha256',
    'nvbes.csrf.v1',
    'verify_slice',
    'v1.',
    'HttpOnly; SameSite=Strict',
    'SameSite=Strict; Path=/',
    '__Host-',
  ]) {
    if (!cookies.includes(needle)) errors.push(`CSRF cookie implementation missing ${needle}`);
  }

  const csrf = readText('apps/identity-service/src/identity.http.middleware.csrf.rs');
  for (const needle of [
    'csrf_guard',
    'is_mutating_method',
    'CSRF_SKIP_PATHS',
    'Sec-Fetch-Site',
    'Sec-Fetch-Mode',
    'Sec-Fetch-Dest',
    'X-CSRF-Token',
    'validate_signed_double_submit_csrf',
    'verify_csrf_token',
    'csrf_token_invalid',
    'csrf_token_mismatch',
    'missing_csrf_header',
    'missing_csrf_token',
  ]) {
    if (!csrf.includes(needle)) errors.push(`CSRF middleware missing ${needle}`);
  }
}

function assertLayeredDefenses() {
  const origin = readText('apps/identity-service/src/identity.http.middleware.origin.rs');
  const cors = readText('apps/identity-service/src/identity.http.cors.rs');
  const login = readText('apps/identity-service/src/identity.domains.auth.routes.login.rs');
  const stepUp = readText(
    'apps/identity-service/src/identity.domains.auth.routes.session_mgmt.step_up.rs',
  );
  const httpClient = readText('libs/ts/http-client/src/index.ts');
  const httpRequestContext = readText('libs/ts/http-client/src/http.request-context.ts');
  const identityCsrf = readText('libs/ts/identity-sdk-web/src/csrf.ts');
  const verifiedFetch = readText('libs/ts/web-runtime/src/verified-fetch.ts');
  const verifiedFetchCsrf = readText('libs/ts/web-runtime/src/verified-fetch.csrf.ts');

  for (const needle of [
    'header::ORIGIN',
    'header::REFERER',
    'allowed_browser_origins.contains',
    'invalid_origin',
  ]) {
    if (!origin.includes(needle)) errors.push(`Origin guard missing ${needle}`);
  }
  for (const needle of [
    'x-csrf-token',
    'x-requested-with',
    'ACCESS_CONTROL_ALLOW_CREDENTIALS',
    'allowed_browser_origins.contains',
    'same_origin',
  ]) {
    if (!cors.includes(needle)) errors.push(`CORS CSRF defense missing ${needle}`);
  }
  for (const needle of [
    'auth_cookie_name_with_user("session"',
    'auth_cookie_name_with_user("csrf_token"',
    'generate_csrf_token(&session_cookie_value, csrf_secret)',
  ]) {
    if (!login.includes(needle)) errors.push(`Login CSRF issuance missing ${needle}`);
  }
  for (const needle of [
    'browser_session_token',
    'auth_cookie_name_with_user("session"',
    'csrf_cookie_name',
    'generate_csrf_token(',
  ]) {
    if (!stepUp.includes(needle)) errors.push(`Step-up CSRF rotation missing ${needle}`);
  }
  for (const needle of [
    'credentials && isMutatingMethod(method)',
    'readCsrfToken(authuser)',
    'X-CSRF-Token',
  ]) {
    if (!httpClient.includes(needle)) errors.push(`HTTP client CSRF behavior missing ${needle}`);
  }
  for (const needle of ['MUTATING_METHODS', 'X-Requested-With']) {
    if (!httpRequestContext.includes(needle)) {
      errors.push(`HTTP request context CSRF behavior missing ${needle}`);
    }
  }
  for (const needle of [
    'readScopedCsrfToken',
    'csrf_token_${authuser}',
    '__Host-csrf_token_${authuser}',
  ]) {
    if (!identityCsrf.includes(needle)) errors.push(`Identity CSRF behavior missing ${needle}`);
  }
  for (const needle of [
    'shouldAttachCsrf',
    'assertAllowedOrigin',
    'X-CSRF-Token',
    'readVerifiedFetchCsrfToken',
  ]) {
    if (!verifiedFetch.includes(needle))
      errors.push(`Verified fetch CSRF behavior missing ${needle}`);
  }
  for (const needle of [
    'readVerifiedFetchCsrfToken',
    'csrf_token_${authuser}',
    '__Host-csrf_token_${authuser}',
  ]) {
    if (!verifiedFetchCsrf.includes(needle))
      errors.push(`Verified fetch CSRF reader missing ${needle}`);
  }
}

function assertTests() {
  const csrf = readText('apps/identity-service/src/identity.http.middleware.csrf.rs');
  const cookies = readText('apps/identity-service/src/identity.http.cookies.rs');
  for (const needle of [
    'fetch_metadata_rejects_cross_site_requests',
    'fetch_metadata_rejects_no_cors_authenticated_requests',
    'signed_double_submit_accepts_token_bound_to_session',
    'signed_double_submit_rejects_header_cookie_mismatch',
    'signed_double_submit_rejects_token_from_other_session',
    'signed_double_submit_rejects_unsigned_legacy_token',
  ]) {
    if (!csrf.includes(needle)) errors.push(`CSRF middleware test missing ${needle}`);
  }
  for (const needle of [
    'csrf_token_is_bound_to_session_token',
    'csrf_token_rejects_unsigned_legacy_values',
  ]) {
    if (!cookies.includes(needle)) errors.push(`CSRF cookie test missing ${needle}`);
  }
}

function assertPackageScript() {
  const pkg = readText('package.json');
  if (!pkg.includes('check:cross-site-request-forgery-prevention')) {
    errors.push('package.json: check:cross-site-request-forgery-prevention script missing');
  }
  if (!pkg.includes('tools/security/check-cross-site-request-forgery-prevention.mjs')) {
    errors.push('package.json: CSRF checker command missing');
  }
}

assertRegistry(readJson(registryPath));
assertImplementation();
assertLayeredDefenses();
assertTests();
assertPackageScript();

if (errors.length > 0) {
  console.error('Cross-Site Request Forgery Prevention controls failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('Cross-Site Request Forgery Prevention controls OK');
