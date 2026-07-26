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
  if (evidenceCount < 24) errors.push(`${registryPath}: expected substantial evidence coverage`);
}

function assertImplementation() {
  const redis = readText('libs/rust/redis/src/redis.session.rs');
  for (const field of [
    'accept_language',
    'accept_encoding',
    'sec_fetch_site',
    'sec_ch_ua_platform',
    'cookie_theft_risk_score',
    'cookie_theft_detected_at',
  ]) {
    if (!redis.includes(field)) errors.push(`CachedSession missing ${field}`);
  }
  if ((redis.match(/serde\(default\)/gu) ?? []).length < 6) {
    errors.push('CachedSession cookie theft fields must use serde(default)');
  }

  const detector = readText(
    'apps/account-service/src/identity.domains.auth.sessions.cookie_theft.rs',
  );
  for (const needle of [
    'SessionRequestProfile',
    'Accept-Language',
    'Sec-Fetch-Site',
    'UserAgentClientHints::from_headers',
    'ip_network_changed',
    'user_agent_family_changed',
    'Reauthenticate',
  ]) {
    if (!detector.includes(needle)) errors.push(`cookie theft detector missing ${needle}`);
  }
}

function assertRequestAwareAuth() {
  const login = readText('apps/account-service/src/identity.domains.auth.routes.login.rs');
  const create = readText(
    'apps/account-service/src/identity.domains.auth.sessions.create.session.rs',
  );
  const auth = readText('apps/account-service/src/identity.domains.auth.sessions.authenticate.rs');
  const refresh = readText(
    'apps/account-service/src/identity.http.middleware.jwt.session_refresh.rs',
  );
  /** @type {Array<[string, string, string[]]>} */
  const requestAwareAuthChecks = [
    ['login routes', login, ['SessionRequestProfile::from_headers', 'request_profile']],
    ['session create', create, ['apply_profile', 'request_profile']],
    [
      'session auth',
      auth,
      ['authenticate_with_request', 'enforce_cookie_theft_mitigation', 'cookie_theft_suspected'],
    ],
    [
      'session middleware',
      refresh,
      [
        'authenticate_session_request',
        'authenticate_with_request',
        'authenticate_browser_session',
        'headers: &HeaderMap',
      ],
    ],
  ];
  for (const [path, text, needles] of requestAwareAuthChecks) {
    for (const needle of needles)
      if (!text.includes(needle)) errors.push(`${path}: missing ${needle}`);
  }
}

function assertReauthenticationAndTelemetry() {
  const auth = readText('apps/account-service/src/identity.domains.auth.sessions.authenticate.rs');
  for (const needle of [
    'session_reauthentication_required',
    'revoke_suspicious_session',
    'revoke_session_refresh_tokens',
    'delete_session',
    'risk::record_event',
    'record_auth_event',
  ]) {
    if (!auth.includes(needle)) errors.push(`session auth missing ${needle}`);
  }
}

function assertCookieAttributes() {
  const cookies = readText('apps/account-service/src/identity.http.cookies.rs');
  for (const needle of ['HttpOnly', 'SameSite=Strict', '; Secure', '__Host-']) {
    if (!cookies.includes(needle)) errors.push(`cookie hardening missing ${needle}`);
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
