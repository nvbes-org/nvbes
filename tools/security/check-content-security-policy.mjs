#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const registryPath = 'docs/security/content-security-policy-controls.json';
const errors = [];
const expectedSchemaVersion = 1;
const webCspPath = 'libs/ts/web-runtime/src/csp.ts';
const apiHeadersPath = 'libs/rust/core/src/security.headers.rs';
const webAppConfigs = [
  'apps/account-web/identity.vite.csp.ts',
  'apps/cloud-web/vite.config.ts',
  'apps/console-web/vite.config.ts',
  'apps/backoffice-web/vite.config.ts',
  'apps/enterprise-web/vite.config.ts',
];
const appHeaderConfigs = [
  'apps/account-web/vite.config.ts',
  'apps/cloud-web/vite.config.ts',
  'apps/console-web/vite.config.ts',
  'apps/backoffice-web/vite.config.ts',
  'apps/enterprise-web/vite.config.ts',
];
const requiredWebDirectives =
  'default-src|script-src|script-src-attr|worker-src|style-src|style-src-elem|style-src-attr|img-src|font-src|connect-src|frame-src|object-src|base-uri|form-action|frame-ancestors|report-uri'.split(
    '|',
  );
const requiredApiDirectives =
  "default-src 'none'|script-src 'none'|script-src-attr 'none'|style-src 'none'|img-src 'none'|font-src 'none'|connect-src 'self'|worker-src 'none'|child-src 'none'|frame-src 'none'|object-src 'none'|manifest-src 'none'|media-src 'none'|base-uri 'self'|form-action 'self'|frame-ancestors 'none'|report-uri /csp-report|report-to nvbes-csp-endpoint".split(
    '|',
  );

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
  if (registry.schemaVersion !== expectedSchemaVersion) {
    errors.push(`${registryPath}: schemaVersion must be ${expectedSchemaVersion}`);
  }

  requireString(registry.source, `${registryPath}.source`);
  requireString(registry.reviewCadence, `${registryPath}.reviewCadence`);

  const requirementIds = new Set();
  const controlIds = new Set();
  let evidenceCount = 0;

  for (const [requirementIndex, requirement] of requireArray(
    registry.requirements,
    `${registryPath}.requirements`,
  ).entries()) {
    const requirementPath = `${registryPath}.requirements[${requirementIndex}]`;
    requireUnique(
      assertId(requirement.id, `${requirementPath}.id`, /^CSP_REQ_\d{3}$/u),
      requirementIds,
      `${requirementPath}.id`,
    );
    requireString(requirement.name, `${requirementPath}.name`);
    requireString(requirement.owasp, `${requirementPath}.owasp`);

    for (const [controlIndex, control] of requireArray(
      requirement.controls,
      `${requirementPath}.controls`,
    ).entries()) {
      const controlPath = `${requirementPath}.controls[${controlIndex}]`;
      requireUnique(
        assertId(control.id, `${controlPath}.id`, /^CSP_CTRL_\d{3}$/u),
        controlIds,
        `${controlPath}.id`,
      );
      requireString(control.name, `${controlPath}.name`);
      requireString(control.description, `${controlPath}.description`);

      for (const [evidenceIndex, evidence] of requireArray(
        control.evidence,
        `${controlPath}.evidence`,
      ).entries()) {
        evidenceCount += requireIncludes(
          requireString(evidence.path, `${controlPath}.evidence[${evidenceIndex}].path`),
          evidence.includes,
          `${controlPath}.evidence[${evidenceIndex}]`,
        );
      }
    }
  }

  if (requirementIds.size < 7) {
    errors.push(`${registryPath}: expected at least 7 CSP requirements`);
  }
  if (controlIds.size < 7) {
    errors.push(`${registryPath}: expected at least 7 CSP controls`);
  }
  if (evidenceCount < 20) {
    errors.push(`${registryPath}: expected substantial evidence coverage`);
  }
}

function assertRequiredWebDirectives() {
  const text = readText(webCspPath);
  for (const directive of requiredWebDirectives) {
    if (!text.includes(directive)) {
      errors.push(`${webCspPath}: missing web CSP directive ${directive}`);
    }
  }

  if (!text.includes('isDev ? ["\'unsafe-inline\'", "\'unsafe-eval\'"] : []')) {
    errors.push(`${webCspPath}: unsafe script sources must be development-only`);
  }
  if (!text.includes('isDev ? ["\'unsafe-inline\'"] : []')) {
    errors.push(`${webCspPath}: unsafe style source must be development-only`);
  }
  if (!text.includes("directive('style-src-attr', [\"'unsafe-inline'\"])")) {
    errors.push(`${webCspPath}: React inline style attributes must remain supported`);
  }
  if (!/directive\('style-src-elem', \[\s*"'self'",\s*"'unsafe-inline'"/u.test(text)) {
    errors.push(`${webCspPath}: runtime-generated SPA style elements must remain supported`);
  }
  if (!text.includes('cspMetaFromHeader') || !text.includes('frame-ancestors')) {
    errors.push(`${webCspPath}: CSP meta conversion must strip header-only directives`);
  }
}

function assertRequiredApiDirectives() {
  const text = readText(apiHeadersPath);
  if (!text.includes('CSP_VALUE')) {
    errors.push(`${apiHeadersPath}: CSP must be declared as a named constant`);
  }

  for (const directive of requiredApiDirectives) {
    if (!text.includes(directive)) {
      errors.push(`${apiHeadersPath}: missing API CSP directive ${directive}`);
    }
  }
}

function assertWebAppsUseSharedCsp() {
  for (const path of webAppConfigs) {
    const text = readText(path);
    if (!text.includes('buildWebCsp')) {
      errors.push(`${path}: must use shared buildWebCsp`);
    }
    if (!text.includes('Content-Security-Policy')) {
      errors.push(`${path}: must emit Content-Security-Policy`);
    }
  }

  for (const path of appHeaderConfigs) {
    const text = readText(path);
    if (!text.includes('Content-Security-Policy')) {
      errors.push(`${path}: must configure Content-Security-Policy response headers`);
    }
    if (!text.includes('Permissions-Policy')) {
      errors.push(`${path}: must configure Permissions-Policy with CSP baseline`);
    }
  }
}

function assertNoProductionUnsafeScriptSources() {
  for (const path of webAppConfigs) {
    const text = readText(path);
    if (text.includes("'unsafe-inline'") || text.includes("'unsafe-eval'")) {
      errors.push(`${path}: unsafe CSP sources must stay inside ${webCspPath}`);
    }
  }
}

function assertNoLocalCspStringLiterals() {
  for (const path of webAppConfigs) {
    const text = readText(path);
    if (/default-src\s+'self';\s*script-src/u.test(text)) {
      errors.push(`${path}: local CSP string literal found; use buildWebCsp`);
    }
  }
}

function assertCspReporting() {
  const routes = readText('apps/account-service/src/identity.http.routes.rs');
  const handler = readText('apps/account-service/src/identity.http.routes.csp_report.rs');
  if (!routes.includes('/csp-report') || !routes.includes('csp_report_handler')) {
    errors.push('apps/account-service/src/identity.http.routes.rs: /csp-report route missing');
  }
  for (const field of [
    'blocked-uri',
    'document-uri',
    'violated-directive',
    'effective-directive',
  ]) {
    if (!handler.includes(field)) {
      errors.push(`apps/account-service/src/identity.http.routes.csp_report.rs: missing ${field}`);
    }
  }
}

function assertNoLegacyCspHeaders() {
  const scanned = [webCspPath, apiHeadersPath, ...webAppConfigs, ...appHeaderConfigs];
  for (const path of scanned) {
    const text = readText(path);
    if (text.includes('X-Content-Security-Policy') || text.includes('X-WebKit-CSP')) {
      errors.push(`${path}: legacy CSP headers are forbidden`);
    }
  }
}

const registry = readJson(registryPath);
assertRegistry(registry);
assertRequiredWebDirectives();
assertRequiredApiDirectives();
assertWebAppsUseSharedCsp();
assertNoProductionUnsafeScriptSources();
assertNoLocalCspStringLiterals();
assertCspReporting();
assertNoLegacyCspHeaders();

if (errors.length > 0) {
  console.error('Content Security Policy controls failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log('Content Security Policy controls OK');
