#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const registryPath = 'docs/security/cross-site-scripting-prevention-controls.json';
const errors = [];
const allowedRawHtmlPath = 'libs/ts/web-runtime/src/safe-html.tsx';
const scanRoots = ['apps', 'libs/ts'];

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
    if (!/^XSS_REQ_\d{3}$/u.test(reqId)) errors.push(`${reqPath}.id: invalid ID ${reqId}`);
    if (reqIds.has(reqId)) errors.push(`${reqPath}.id: duplicate ${reqId}`);
    reqIds.add(reqId);
    requireString(req.name, `${reqPath}.name`);
    requireString(req.owasp, `${reqPath}.owasp`);
    for (const [ctrlIndex, ctrl] of requireArray(req.controls, `${reqPath}.controls`).entries()) {
      const ctrlPath = `${reqPath}.controls[${ctrlIndex}]`;
      const ctrlId = requireString(ctrl.id, `${ctrlPath}.id`);
      if (!/^XSS_CTRL_\d{3}$/u.test(ctrlId)) errors.push(`${ctrlPath}.id: invalid ID ${ctrlId}`);
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
  if (evidenceCount < 30) errors.push(`${registryPath}: expected substantial evidence coverage`);
}

function assertImplementation() {
  const safeHtml = readText('libs/ts/web-runtime/src/safe-html.tsx');
  for (const needle of [
    'export type SafeHtml',
    'sanitizeHtml',
    'VerifiedHtml',
    'BLOCKED_CONTENT_TAGS',
    'EVENT_HANDLER_ATTRIBUTES',
    'DANGEROUS_ATTRIBUTES',
    'UNSAFE_URL_ATTRIBUTES',
  ]) {
    if (!safeHtml.includes(needle)) errors.push(`safe-html missing ${needle}`);
  }

  const safeUrl = readText('libs/ts/web-runtime/src/safe-url.ts');
  for (const needle of [
    'export type SafeUrl',
    'sanitizeUrlForAttribute',
    'javascript:',
    'vbscript:',
    'data:',
    'containsEncodedControlChars',
  ]) {
    if (!safeUrl.includes(needle)) errors.push(`safe-url missing ${needle}`);
  }

  const safeCss = readText('libs/ts/web-runtime/src/safe-css.ts');
  for (const needle of [
    'UNSAFE_STYLE_ELEMENT_PATTERNS',
    'sanitizeStyleElementCss',
    '@import',
    'url',
  ]) {
    if (!safeCss.includes(needle)) errors.push(`safe-css missing ${needle}`);
  }
}

function assertLayeredDefenses() {
  const consent = readText('apps/identity-web/src/pages/LoginPageConsent.tsx');
  for (const needle of [
    'sanitizeUrlForAttribute',
    'sanitizeStyleElementCss',
    'safeLogoUrl',
    'safePrivacyUrl',
    'safeTermsUrl',
    'safeSupportUrl',
  ]) {
    if (!consent.includes(needle)) errors.push(`Login consent XSS defense missing ${needle}`);
  }
  for (const unsafeNeedle of [
    'src={logoUrl}',
    'href={privacyUrl}',
    'href={termsUrl}',
    'href={supportUrl}',
    '<style>{customCss}</style>',
  ]) {
    if (consent.includes(unsafeNeedle))
      errors.push(`Login consent still uses unsafe sink ${unsafeNeedle}`);
  }

  const developerConsent = readText('apps/developer-service/src/developer.grpc.consent.rs');
  for (const needle of ['https_url_option', 'safe_css_option', 'url.scheme() != "https"']) {
    if (!developerConsent.includes(needle))
      errors.push(`Developer consent validation missing ${needle}`);
  }

  const csp = readText('libs/ts/web-runtime/src/csp.ts');
  for (const needle of ['script-src-attr', 'object-src', 'base-uri', 'form-action']) {
    if (!csp.includes(needle)) errors.push(`Web CSP missing ${needle}`);
  }
}

function listFiles(root) {
  if (!existsSync(root)) return [];
  const files = [];
  for (const entry of readdirSync(root)) {
    if (['node_modules', 'dist', 'target'].includes(entry)) continue;
    const path = join(root, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) files.push(...listFiles(path));
    if (stat.isFile() && /\.(tsx?|jsx?)$/u.test(path) && !path.endsWith('.gen.ts'))
      files.push(path);
  }
  return files;
}

function assertNoUnsafeDomSinks() {
  for (const root of scanRoots) {
    for (const path of listFiles(root)) {
      const normalized = path.replaceAll('\\', '/');
      const text = readText(normalized);
      if (normalized !== allowedRawHtmlPath && text.includes('dangerouslySetInnerHTML')) {
        errors.push(`${normalized}: dangerouslySetInnerHTML must go through VerifiedHtml`);
      }
      for (const sink of ['.innerHTML', '.outerHTML', 'insertAdjacentHTML']) {
        if (text.includes(sink)) errors.push(`${normalized}: unsafe DOM sink ${sink}`);
      }
      if (/\beval\s*\(/u.test(text) || /\bnew\s+Function\s*\(/u.test(text)) {
        errors.push(`${normalized}: dynamic JavaScript execution is forbidden`);
      }
    }
  }
}

function assertTestsAndPackageScript() {
  const tests = readText('libs/ts/web-runtime/src/xss-prevention.test.ts');
  for (const needle of [
    'sanitizes executable HTML',
    'rejects scriptable URLs',
    'rejects unsafe CSS',
  ]) {
    if (!tests.includes(needle)) errors.push(`XSS tests missing ${needle}`);
  }
  const pkg = readText('package.json');
  if (!pkg.includes('check:cross-site-scripting-prevention')) {
    errors.push('package.json: check:cross-site-scripting-prevention script missing');
  }
}

assertRegistry(readJson(registryPath));
assertImplementation();
assertLayeredDefenses();
assertNoUnsafeDomSinks();
assertTestsAndPackageScript();

if (errors.length > 0) {
  console.error('Cross Site Scripting Prevention controls failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('Cross Site Scripting Prevention controls OK');
