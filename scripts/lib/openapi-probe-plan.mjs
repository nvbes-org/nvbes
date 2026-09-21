const PROXIED_PREFIXES = ['/api', '/auth', '/oauth', '/csp-report'];
const MUTATING_METHODS = new Set(['post', 'put', 'patch', 'delete']);
const JSON_CONTENT_TYPES = new Set([
  'application/json',
  'application/merge-patch+json',
  'application/ld+json',
]);
const SAFE_PROTECTED_MUTATING_OPERATIONS = new Set([
  'PATCH /auth/me',
  'PUT /auth/me/preferences',
  'PUT /auth/me/notifications',
  'POST /authz/decision',
]);
const ALWAYS_SKIP_OPERATIONS = new Set([
  'POST /auth/logout',
  'DELETE /auth/sessions/{sessionId}',
  'POST /auth/sessions/revoke-others',
  'POST /auth/me/delete',
]);
const ALWAYS_SKIP_PATH_SUBSTRINGS = [
  '/webhooks/',
  '/oauth/callback',
  '/saml/acs',
  '/auth/challenge/',
  '/auth/password/',
  '/auth/verify-email',
  '/auth/register',
];

export const ROUTABLE_METHODS = ['get', 'post', 'put', 'patch', 'delete', 'head', 'options'];

export function deref(value, spec) {
  if (value && typeof value === 'object' && '$ref' in value) {
    const resolved = resolveRef(value.$ref, spec);
    if (!resolved) {
      throw new Error(`unable to resolve OpenAPI ref: ${value.$ref}`);
    }
    return resolved;
  }
  return value;
}

export function collectParameters(pathItem, operation, spec) {
  return [...(pathItem.parameters ?? []), ...(operation.parameters ?? [])]
    .map((entry) => deref(entry, spec))
    .filter(Boolean);
}

export function buildPath(pathPattern, parameters, authContext, spec) {
  const pathParams = new Map(
    parameters
      .filter((parameter) => parameter.in === 'path')
      .map((parameter) => [parameter.name, parameter]),
  );
  const concretePath = pathPattern.replace(/\{([^{}\r\n]+)\}/gu, (_match, rawName) => {
    const parameter = pathParams.get(rawName);
    return encodeURIComponent(samplePathParam(parameter ?? { name: rawName }, authContext, spec));
  });

  const searchParams = new URLSearchParams();
  for (const parameter of parameters.filter((entry) => entry.in === 'query')) {
    const contextual = valueFromAuthContext(parameter.name, authContext);
    if (contextual) {
      searchParams.set(parameter.name, contextual);
      continue;
    }
    const schema = deref(parameter.schema ?? parameter.content?.['application/json']?.schema, spec);
    const sample = sampleFromSchema(schema, spec);
    if (sample !== null && sample !== undefined) {
      searchParams.set(
        parameter.name,
        typeof sample === 'string' ? sample : JSON.stringify(sample),
      );
    }
  }
  const query = searchParams.toString();
  return query ? `${concretePath}?${query}` : concretePath;
}

export function buildRequestInit(method, operation, spec, authContext, isProtected) {
  const headers = { Accept: '*/*' };
  if (isProtected && authContext?.token) {
    headers.Authorization = `Bearer ${authContext.token}`;
  }

  let body;
  if (MUTATING_METHODS.has(method)) {
    headers.Origin = 'http://localhost:3001';
    headers['Content-Type'] = 'application/json';
    body = '{}';

    const requestBody = deref(operation.requestBody, spec);
    const content = requestBody?.content ?? {};
    const contentType = pickContentType(content);
    if (contentType) {
      headers['Content-Type'] = contentType;
      const schema = deref(content[contentType]?.schema, spec);
      const sample = enrichSampleWithAuthContext(sampleFromSchema(schema, spec), authContext);
      if (contentType === 'application/x-www-form-urlencoded') {
        body =
          sample && typeof sample === 'object' && !Array.isArray(sample)
            ? new URLSearchParams(
                Object.entries(sample).map(([key, value]) => [key, String(value ?? '')]),
              ).toString()
            : 'smoke=true';
      } else if (contentType === 'text/plain') {
        body = typeof sample === 'string' ? sample : 'smoke';
      } else if (sample === null || sample === undefined) {
        body = '{}';
      } else {
        body = typeof sample === 'string' ? sample : JSON.stringify(sample);
      }
    }
  }
  return { headers, body };
}

export function shouldSmokeThroughProxy(pathPattern) {
  return PROXIED_PREFIXES.some((prefix) => pathPattern.startsWith(prefix));
}

export function isProtectedOperation(operation, globalSecurity) {
  if (Array.isArray(operation.security)) {
    return operation.security.length > 0;
  }
  return Array.isArray(globalSecurity) && globalSecurity.length > 0;
}

export function shouldSkipOperation(
  pathPattern,
  method,
  operation,
  globalSecurity,
  seededAuthEnabled,
) {
  const key = operationKey(method, pathPattern);
  if (ALWAYS_SKIP_OPERATIONS.has(key)) {
    return `${key} [destructive]`;
  }
  if (ALWAYS_SKIP_PATH_SUBSTRINGS.some((segment) => pathPattern.includes(segment))) {
    return `${key} [non-smokeable]`;
  }

  const protectedOperation = isProtectedOperation(operation, globalSecurity);
  if (protectedOperation && !seededAuthEnabled) {
    return `${key} [protected]`;
  }
  if (
    protectedOperation &&
    MUTATING_METHODS.has(method) &&
    !SAFE_PROTECTED_MUTATING_OPERATIONS.has(key)
  ) {
    return `${key} [protected-stateful]`;
  }
  if (!protectedOperation && MUTATING_METHODS.has(method) && pathPattern.includes('{')) {
    return `${key} [stateful]`;
  }
  return null;
}

function resolveRef(ref, spec) {
  if (typeof ref !== 'string' || !ref.startsWith('#/')) {
    return undefined;
  }
  return ref
    .slice(2)
    .split('/')
    .reduce((current, segment) => current?.[segment], spec);
}

function operationKey(method, pathPattern) {
  return `${method.toUpperCase()} ${pathPattern}`;
}

function sampleFromSchema(schema, spec) {
  const resolved = deref(schema, spec);
  if (!resolved || typeof resolved !== 'object') return 'smoke';
  for (const union of ['oneOf', 'anyOf', 'allOf']) {
    if (Array.isArray(resolved[union]) && resolved[union].length > 0) {
      return sampleFromSchema(resolved[union][0], spec);
    }
  }
  for (const explicit of ['const', 'default', 'example']) {
    if (resolved[explicit] !== undefined) return resolved[explicit];
  }
  if (resolved.nullable || resolved.type === 'null') return null;
  if (Array.isArray(resolved.enum) && resolved.enum.length > 0) return resolved.enum[0];
  if (['integer', 'number'].includes(resolved.type)) return 1;
  if (resolved.type === 'boolean') return true;
  if (resolved.format === 'uuid') return '00000000-0000-0000-0000-000000000000';
  if (resolved.format === 'date') return '2024-01-01';
  if (resolved.format === 'date-time') return '2024-01-01T00:00:00Z';
  if (resolved.format === 'uri') return 'https://example.com';
  if (resolved.type === 'array') return [sampleFromSchema(resolved.items, spec)];
  if (resolved.type === 'object') {
    const properties =
      resolved.properties && typeof resolved.properties === 'object'
        ? Object.entries(resolved.properties)
        : [];
    return Object.fromEntries(
      properties.slice(0, 4).map(([key, value]) => [key, sampleFromSchema(value, spec)]),
    );
  }
  return 'smoke';
}

function valueFromAuthContext(name, authContext) {
  if (!authContext) return undefined;
  const normalized = name.toLowerCase();
  if (normalized.includes('workspace')) return authContext.workspaceId;
  if (normalized.includes('session')) return authContext.sessionId;
  if (normalized.includes('user') || normalized.includes('principal')) {
    return authContext.userId;
  }
  return undefined;
}

function enrichSampleWithAuthContext(sample, authContext) {
  if (!authContext || !sample || typeof sample !== 'object' || Array.isArray(sample)) {
    return sample;
  }
  const patched = { ...sample };
  for (const key of Object.keys(patched)) {
    const contextual = valueFromAuthContext(key, authContext);
    if (contextual) patched[key] = contextual;
  }
  return patched;
}

function samplePathParam(parameter, authContext, spec) {
  const contextual = valueFromAuthContext(parameter.name, authContext);
  if (contextual) return contextual;
  const schema = deref(parameter.schema ?? parameter.content?.['application/json']?.schema, spec);
  if (schema) {
    const sample = sampleFromSchema(schema, spec);
    if (typeof sample === 'string') return sample;
    return sample === null || sample === undefined ? 'smoke' : String(sample);
  }
  const name = parameter.name.toLowerCase();
  if (name.endsWith('id') || name.includes('uuid')) {
    return '00000000-0000-0000-0000-000000000000';
  }
  if (['page', 'limit', 'offset'].some((part) => name.includes(part))) return '1';
  return 'smoke';
}

function pickContentType(content = {}) {
  const available = Object.keys(content);
  return (
    available.find((type) => JSON_CONTENT_TYPES.has(type)) ??
    available.find((type) => type.startsWith('application/') && type.includes('json')) ??
    available[0]
  );
}
