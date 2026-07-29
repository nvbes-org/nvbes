import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { pathToFileURL } from 'node:url';

import {
  parseOriginAllowlist,
  productionOrigins,
  validateDastTarget,
  verifyPublicResolution,
} from '../security/validate-dast-targets.mjs';

const MAX_CREDENTIAL_LENGTH = 16_384;
const MAX_RESPONSE_LENGTH = 1_048_576;
const REQUEST_TIMEOUT_MS = 15_000;
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;

export async function checkDastAuthentication({
  env = process.env,
  fetchImpl = fetch,
  resolver,
  verifiedAt = new Date(),
} = {}) {
  const allowedOrigins = parseOriginAllowlist(env.DAST_ALLOWED_ORIGINS ?? '');
  const forbiddenOrigins = productionOrigins(env.DAST_PRODUCTION_ORIGINS);
  const expectedSubject = requiredUuid(env, 'DAST_EXPECTED_SUBJECT_ID');
  const expectedTenant = requiredUuid(env, 'DAST_EXPECTED_TENANT_ID');

  const accountCookie = requiredCredential(env, 'DAST_ACCOUNT_SESSION_COOKIE', 'Cookie');
  const apiAuthorization = requiredCredential(env, 'DAST_API_AUTHORIZATION', 'Authorization');
  const backofficeCookie = requiredCredential(env, 'DAST_BACKOFFICE_SESSION_COOKIE', 'Cookie');

  const checks = [
    {
      assertions: [
        ['user.id', expectedSubject],
        ['current_tenant_id', expectedTenant],
      ],
      baseUrl: requiredValue(env, 'ACCOUNT_URL'),
      headers: { Cookie: accountCookie },
      path: '/auth/me',
      route: '/auth/me',
      surface: 'account',
    },
    {
      assertions: [
        ['sub', expectedSubject],
        ['tenant_id', expectedTenant],
      ],
      baseUrl: requiredValue(env, 'API_URL'),
      headers: { Authorization: apiAuthorization },
      path: '/oauth/userinfo',
      route: '/oauth/userinfo',
      surface: 'api',
    },
    {
      assertions: [
        ['principal_id', expectedSubject],
        ['tenant_id', expectedTenant],
      ],
      baseUrl: requiredValue(env, 'BACKOFFICE_URL'),
      headers: { Cookie: backofficeCookie },
      path: `/admin/users/${encodeURIComponent(expectedSubject)}`,
      route: '/admin/users/{syntheticSubject}',
      surface: 'backoffice',
    },
  ];

  const evidence = [];
  for (const check of checks) {
    const baseUrl = validateDastTarget(check.baseUrl, allowedOrigins, forbiddenOrigins);
    const endpoint = validateDastTarget(
      new URL(check.path, `${baseUrl.origin}/`).toString(),
      allowedOrigins,
      forbiddenOrigins,
    );
    await verifyPublicResolution(endpoint, resolver);
    const body = await requestProtectedJson(check.surface, endpoint, check.headers, fetchImpl);
    for (const [claimPath, expected] of check.assertions) {
      requireClaim(check.surface, body, claimPath, expected);
    }
    evidence.push({
      assertions: check.assertions.map(([claimPath]) => `${claimPath}:fixture-match`),
      origin: endpoint.origin,
      route: check.route,
      status: 200,
      surface: check.surface,
    });
  }

  return {
    accountAndApiIdentityBoundToFixture: true,
    backofficeCredentialAuthorizedForFixture: true,
    checks: evidence,
    credentialMaterialRecorded: false,
    schemaVersion: 2,
    verifiedAt: verifiedAt.toISOString(),
  };
}

async function requestProtectedJson(surface, url, headers, fetchImpl) {
  let response;
  try {
    response = await fetchImpl(url, {
      headers,
      method: 'GET',
      redirect: 'manual',
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
  } catch {
    throw new Error(`${surface} authenticated preflight request failed`);
  }

  if (response.status !== 200) {
    throw new Error(
      `${surface} authenticated preflight expected HTTP 200 and received ${response.status}`,
    );
  }
  const contentType = response.headers.get('content-type')?.toLowerCase() ?? '';
  if (!contentType.startsWith('application/json')) {
    throw new Error(`${surface} authenticated preflight did not return JSON`);
  }
  const declaredLength = Number(response.headers.get('content-length') ?? 0);
  if (Number.isFinite(declaredLength) && declaredLength > MAX_RESPONSE_LENGTH) {
    throw new Error(`${surface} authenticated preflight response is too large`);
  }

  let text;
  try {
    text = await response.text();
  } catch {
    throw new Error(`${surface} authenticated preflight response could not be read`);
  }
  if (text.length > MAX_RESPONSE_LENGTH) {
    throw new Error(`${surface} authenticated preflight response is too large`);
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`${surface} authenticated preflight returned invalid JSON`);
  }
}

function requireClaim(surface, body, claimPath, expected) {
  let value = body;
  for (const segment of claimPath.split('.')) {
    if (!isRecord(value) || !(segment in value)) {
      throw new Error(`${surface} authenticated preflight is missing ${claimPath}`);
    }
    value = value[segment];
  }
  if (typeof value !== 'string' || value.toLowerCase() !== expected) {
    throw new Error(`${surface} authenticated preflight rejected ${claimPath}`);
  }
}

function requiredCredential(env, name, expectedScheme) {
  const value = requiredValue(env, name);
  if (
    value.length > MAX_CREDENTIAL_LENGTH ||
    containsAsciiControl(value) ||
    (expectedScheme === 'Cookie' && !value.includes('=')) ||
    (expectedScheme === 'Authorization' && !value.startsWith('Bearer '))
  ) {
    throw new Error(`${name} is not a valid ${expectedScheme} header value`);
  }
  return value;
}

function containsAsciiControl(value) {
  for (const character of value) {
    const codePoint = character.codePointAt(0);
    if (codePoint !== undefined && (codePoint <= 31 || codePoint === 127)) {
      return true;
    }
  }
  return false;
}

function requiredUuid(env, name) {
  const value = requiredValue(env, name).toLowerCase();
  if (!UUID_PATTERN.test(value)) {
    throw new Error(`${name} must be a canonical UUID`);
  }
  return value;
}

function requiredValue(env, name) {
  const value = env[name]?.trim();
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

async function main() {
  const evidence = await checkDastAuthentication();
  const outputPath = process.env.DAST_PREFLIGHT_EVIDENCE_PATH ?? 'zap-reports/auth-preflight.json';
  await mkdir(dirname(outputPath), { recursive: true });
  await writeFile(outputPath, `${JSON.stringify(evidence, null, 2)}\n`, {
    encoding: 'utf8',
    mode: 0o600,
  });
  process.stdout.write(
    `Authenticated DAST preflight passed for ${evidence.checks.length} staging surfaces.\n`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : 'DAST preflight failed'}\n`);
    process.exitCode = 1;
  });
}
