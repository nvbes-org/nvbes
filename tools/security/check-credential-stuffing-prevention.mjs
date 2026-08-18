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
    if (!/^CSPREV_REQ_\d{3}$/u.test(reqId)) errors.push(`${reqPath}.id: invalid ID ${reqId}`);
    if (reqIds.has(reqId)) errors.push(`${reqPath}.id: duplicate ${reqId}`);
    reqIds.add(reqId);
    requireString(req.name, `${reqPath}.name`);
    requireString(req.owasp, `${reqPath}.owasp`);
    for (const [ctrlIndex, ctrl] of requireArray(req.controls, `${reqPath}.controls`).entries()) {
      const ctrlPath = `${reqPath}.controls[${ctrlIndex}]`;
      const ctrlId = requireString(ctrl.id, `${ctrlPath}.id`);
      if (!/^CSPREV_CTRL_\d{3}$/u.test(ctrlId)) {
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
  if (reqIds.size < 8) errors.push(`${registryPath}: expected at least 8 requirements`);
  if (ctrlIds.size < 8) errors.push(`${registryPath}: expected at least 8 controls`);
  if (evidenceCount < 28) errors.push(`${registryPath}: expected substantial evidence coverage`);
}

function assertImplementation() {
  const redis = readText('libs/rust/redis/src/redis.credential_stuffing.rs');
  for (const needle of [
    'record_failed_login',
    'assess_current',
    'ip_failed_accounts_short',
    'account_failed_sources_short',
    'CredentialStuffingDecision::StepUp',
    'CredentialStuffingDecision::Block',
    'score >= 70',
  ]) {
    if (!redis.includes(needle)) errors.push(`redis credential stuffing missing ${needle}`);
  }
  const lib = readText('libs/rust/redis/src/lib.rs');
  if (!lib.includes('redis.credential_stuffing.rs')) errors.push('nvbes-redis module not exported');
}

function assertLayeredDefenses() {
  const verify = readText(
    'apps/identity-service/src/identity.domains.auth.sessions.create.verify.rs',
  );
  const stuffing = readText(
    'apps/identity-service/src/identity.domains.auth.credential_stuffing.rs',
  );
  const flow = readText(
    'apps/identity-service/src/identity.domains.auth.routes.login.identifier_flow.rs',
  );
  const guard = readText(
    'apps/identity-service/src/identity.domains.auth.routes.login.identifier_guard.rs',
  );
  const exposed = readText('apps/identity-service/src/identity.domains.auth.exposed_credentials.rs');
  const throttle = readText('apps/identity-service/src/identity.domains.auth.login.throttle.rs');

  for (const needle of [
    'record_failed_login',
    'unknown_account',
    'invalid_password',
    'successful_password_risk_score',
    'risk_score: risk_score + stuffing_score',
  ]) {
    if (!verify.includes(needle)) errors.push(`password verification missing ${needle}`);
  }
  for (const needle of [
    'credential_stuffing_suspected',
    'credential_stuffing_blocked',
    'apply_tarpit',
  ]) {
    if (!stuffing.includes(needle)) errors.push(`account credential stuffing missing ${needle}`);
  }
  for (const needle of ['risk_score >= RISK_STEP_UP_THRESHOLD', 'resolve_mfa_challenge_methods']) {
    if (!flow.includes(needle)) errors.push(`MFA step-up missing ${needle}`);
  }
  for (const needle of [
    'require_pow_solution',
    'bot_guard',
    'decoy_link_clicked',
    'extract_http_signals',
  ]) {
    if (!guard.includes(needle)) errors.push(`identifier guard missing ${needle}`);
  }
  for (const needle of ['Exposed-Credential-Check', 'password_leaked', 'password_compromised']) {
    if (!exposed.includes(needle)) errors.push(`exposed credential control missing ${needle}`);
  }
  for (const needle of ['pre_lookup_rules', 'tenant_rule', 'max_hits: 60', 'max_hits: 12']) {
    if (!throttle.includes(needle)) errors.push(`login throttle missing ${needle}`);
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
