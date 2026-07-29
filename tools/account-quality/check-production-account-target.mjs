import { lookup } from 'node:dns/promises';
import { isIP } from 'node:net';
import { pathToFileURL } from 'node:url';
import { assertPublicNetworkAddress } from '../load-tests/account/target-address-policy.js';

const RELEASE_PATTERN = /^[a-f0-9]{40,64}$/u;
const MAX_HEALTH_BYTES = 65_536;

export async function checkProductionAccountTarget({
  webBaseUrl,
  apiBaseUrl,
  allowedOrigins,
  expectedRelease,
  fetchImpl = fetch,
  resolver = lookup,
}) {
  assert(
    RELEASE_PATTERN.test(expectedRelease ?? ''),
    'production release must be an immutable SHA',
  );
  const allowlist = parseProductionAllowlist(allowedOrigins);
  const web = validateProductionOrigin(webBaseUrl, allowlist, 'production web');
  const api = validateProductionOrigin(apiBaseUrl, allowlist, 'production API');
  await Promise.all([verifyDns(web, resolver), verifyDns(api, resolver)]);
  await requireNonRedirectingSuccess(web, 'production web', fetchImpl);
  const health = new URL('/health', `${api.origin}/`);
  const response = await requireNonRedirectingSuccess(health, 'production API health', fetchImpl);
  const body = await readHealth(response);
  assert(body.status === 'ok', 'production API health status is not ok');
  assert(body.release_id === expectedRelease, 'production API release does not match expected SHA');
  return { apiOrigin: api.origin, release: body.release_id, webOrigin: web.origin };
}

function parseProductionAllowlist(source) {
  const entries = (source ?? '')
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean);
  assert(entries.length > 0, 'NVBES_PRODUCTION_ALLOWED_ORIGINS is required');
  return new Set(
    entries.map((entry) => validateBareHttpsOrigin(entry, 'production allowlist').origin),
  );
}

function validateProductionOrigin(source, allowlist, label) {
  const url = validateBareHttpsOrigin(source, label);
  assert(allowlist.has(url.origin), `${label} is not in the exact production allowlist`);
  assert(!isLocalHostname(url.hostname), `${label} must not target a local hostname`);
  if (isIP(url.hostname) > 0) {
    assertPublicNetworkAddress(url.hostname);
  }
  return url;
}

function validateBareHttpsOrigin(source, label) {
  let url;
  try {
    url = new URL(source);
  } catch {
    throw new Error(`${label} must be a valid URL`);
  }
  assert(
    url.protocol === 'https:' &&
      !url.username &&
      !url.password &&
      url.pathname === '/' &&
      !url.search &&
      !url.hash,
    `${label} must be a bare HTTPS origin`,
  );
  return url;
}

async function verifyDns(url, resolver) {
  let addresses;
  try {
    addresses = await resolver(url.hostname, { all: true, verbatim: true });
  } catch {
    throw new Error('production target DNS resolution failed');
  }
  assert(Array.isArray(addresses) && addresses.length > 0, 'production target DNS is empty');
  for (const answer of addresses) {
    assertPublicNetworkAddress(answer.address);
  }
}

async function requireNonRedirectingSuccess(url, label, fetchImpl) {
  let response;
  try {
    response = await fetchImpl(url, {
      method: 'GET',
      redirect: 'manual',
      signal: AbortSignal.timeout(15_000),
    });
  } catch {
    throw new Error(`${label} request failed`);
  }
  assert(
    response.status >= 200 && response.status < 300,
    `${label} expected 2xx and received ${response.status}`,
  );
  return response;
}

async function readHealth(response) {
  const contentType = response.headers.get('content-type')?.toLowerCase() ?? '';
  assert(contentType.startsWith('application/json'), 'production API health must return JSON');
  const declaredLength = Number(response.headers.get('content-length') ?? 0);
  assert(
    !Number.isFinite(declaredLength) || declaredLength <= MAX_HEALTH_BYTES,
    'production API health response is too large',
  );
  const text = await response.text();
  assert(text.length <= MAX_HEALTH_BYTES, 'production API health response is too large');
  try {
    const body = JSON.parse(text);
    assert(isRecord(body), 'production API health must return an object');
    return body;
  } catch (error) {
    if (error instanceof SyntaxError) {
      throw new Error('production API health returned invalid JSON');
    }
    throw error;
  }
}

function isLocalHostname(hostname) {
  const value = hostname.toLowerCase();
  return (
    value === 'localhost' ||
    value.endsWith('.localhost') ||
    value.endsWith('.local') ||
    value.endsWith('.internal')
  );
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function main() {
  const result = await checkProductionAccountTarget({
    allowedOrigins: process.env.NVBES_PRODUCTION_ALLOWED_ORIGINS,
    apiBaseUrl: process.env.NVBES_PRODUCTION_API_BASE_URL,
    expectedRelease: process.env.NVBES_RELEASE_SHA,
    webBaseUrl: process.env.NVBES_PRODUCTION_WEB_BASE_URL,
  });
  process.stdout.write(
    `Verified production Account targets ${result.webOrigin} and ${result.apiOrigin} at ${result.release}.\n`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'production target verification failed'}\n`,
    );
    process.exitCode = 1;
  });
}
