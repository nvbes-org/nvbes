const POLICY_NAME = 'nvbes#default';
const DEFAULT_POLICY_NAME = 'default';
const SERVICE_WORKER_PATH = '/sw.js';

export interface NvbesTrustedScriptUrl {
  toString(): string;
}

interface NvbesTrustedTypePolicy {
  createHTML(value: string): TrustedHTML;
  createScriptURL(value: string): NvbesTrustedScriptUrl;
}

interface NvbesTrustedTypePolicyFactory {
  createPolicy(
    name: string,
    rules: {
      createHTML(value: string): string;
      createScriptURL?(value: string): string;
    },
  ): NvbesTrustedTypePolicy;
}

type GlobalWithTrustedTypes = typeof globalThis & {
  trustedTypes?: NvbesTrustedTypePolicyFactory;
  __nvbesDefaultTrustedTypesPolicy?: NvbesTrustedTypePolicy | null;
  __nvbesTrustedTypesPolicy?: NvbesTrustedTypePolicy | null;
};

export function installDefaultTrustedTypesPolicy(): void {
  const runtime = globalThis as GlobalWithTrustedTypes;
  if (runtime.__nvbesDefaultTrustedTypesPolicy !== undefined) {
    return;
  }

  const factory = runtime.trustedTypes;
  if (!factory) {
    runtime.__nvbesDefaultTrustedTypesPolicy = null;
    return;
  }

  runtime.__nvbesDefaultTrustedTypesPolicy = factory.createPolicy(DEFAULT_POLICY_NAME, {
    createHTML: allowMarkupFreeHtml,
    createScriptURL: (value) => normalizeSameOriginScriptUrl(value, 'default script'),
  });
}

export function createTrustedHtml(value: string): string | TrustedHTML {
  const policy = getTrustedTypesPolicy();
  return policy ? policy.createHTML(value) : value;
}

export function createTrustedServiceWorkerScriptUrl(value: string): string | NvbesTrustedScriptUrl {
  const normalized = normalizeServiceWorkerScriptUrl(value);
  const policy = getTrustedTypesPolicy();
  return policy ? policy.createScriptURL(normalized) : normalized;
}

export function createTrustedWorkerScriptUrl(value: string | URL): string | NvbesTrustedScriptUrl {
  const normalized = normalizeSameOriginScriptUrl(value.toString(), 'worker script');
  const policy = getTrustedTypesPolicy();
  return policy ? policy.createScriptURL(normalized) : normalized;
}

function getTrustedTypesPolicy(): NvbesTrustedTypePolicy | null {
  const runtime = globalThis as GlobalWithTrustedTypes;
  if (runtime.__nvbesTrustedTypesPolicy !== undefined) {
    return runtime.__nvbesTrustedTypesPolicy;
  }

  const factory = runtime.trustedTypes;
  if (!factory) {
    runtime.__nvbesTrustedTypesPolicy = null;
    return null;
  }

  const policy = factory.createPolicy(POLICY_NAME, {
    createHTML: (value) => value,
    createScriptURL: (value) => normalizeSameOriginScriptUrl(value, 'trusted script'),
  });
  runtime.__nvbesTrustedTypesPolicy = policy;
  return policy;
}

function allowMarkupFreeHtml(value: string): string {
  if (value.includes('<') || value.includes('>')) {
    throw new TypeError('Default Trusted Types policy only accepts markup-free text.');
  }
  return value;
}

function normalizeServiceWorkerScriptUrl(value: string): string {
  if (typeof location === 'undefined') {
    if (value !== SERVICE_WORKER_PATH) {
      throw new TypeError('Invalid service worker script URL.');
    }
    return value;
  }

  const url = new URL(value, location.href);
  if (
    url.origin !== location.origin ||
    url.pathname !== SERVICE_WORKER_PATH ||
    url.search ||
    url.hash
  ) {
    throw new TypeError('Invalid service worker script URL.');
  }

  return SERVICE_WORKER_PATH;
}

function normalizeSameOriginScriptUrl(value: string, label: string): string {
  if (typeof location === 'undefined') {
    throw new TypeError(`Invalid ${label} URL.`);
  }

  const url = new URL(value, location.href);
  if (
    url.origin !== location.origin ||
    !['http:', 'https:'].includes(url.protocol) ||
    url.username ||
    url.password ||
    url.hash
  ) {
    throw new TypeError(`Invalid ${label} URL.`);
  }

  return url.href;
}
