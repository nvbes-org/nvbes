import { lookup } from 'node:dns/promises';
import { isIP } from 'node:net';
import { pathToFileURL } from 'node:url';
import { assertPublicNetworkAddress } from '../load-tests/account/target-address-policy.js';

const DEFAULT_PRODUCTION_ORIGINS = new Set([
  'https://account.nvbes.fr',
  'https://api.nvbes.fr',
  'https://backoffice.nvbes.fr',
]);

export function parseOriginAllowlist(raw, label = 'DAST_ALLOWED_ORIGINS') {
  const entries = raw
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean);
  if (entries.length === 0) {
    throw new Error(`${label} must contain at least one exact staging origin`);
  }

  return new Set(
    entries.map((entry) => {
      const url = new URL(entry);
      if (
        url.protocol !== 'https:' ||
        url.username ||
        url.password ||
        url.pathname !== '/' ||
        url.search ||
        url.hash
      ) {
        throw new Error(`${label} entries must be bare HTTPS origins: ${entry}`);
      }
      return url.origin;
    }),
  );
}

export function productionOrigins(raw = '') {
  const configured = raw.trim() ? parseOriginAllowlist(raw, 'DAST_PRODUCTION_ORIGINS') : new Set();
  return new Set([...DEFAULT_PRODUCTION_ORIGINS, ...configured]);
}

export function validateDastTarget(raw, allowedOrigins, forbiddenOrigins) {
  const url = new URL(raw);
  if (url.protocol !== 'https:') {
    throw new Error(`DAST targets must use HTTPS: ${raw}`);
  }
  if (url.username || url.password) {
    throw new Error(`DAST targets must not contain URL credentials: ${raw}`);
  }
  if (url.hash) {
    throw new Error(`DAST targets must not contain fragments: ${raw}`);
  }
  if (forbiddenOrigins.has(url.origin)) {
    throw new Error(`Production DAST target is forbidden: ${url.origin}`);
  }
  if (!allowedOrigins.has(url.origin)) {
    throw new Error(`DAST target is not in the exact staging allowlist: ${url.origin}`);
  }
  if (isLocalHostname(url.hostname) || (isIP(url.hostname) > 0 && !isPublicAddress(url.hostname))) {
    throw new Error(`DAST target resolves syntactically to a non-public host: ${url.hostname}`);
  }
  return url;
}

export async function verifyPublicResolution(url, resolver = lookup) {
  const addresses = await resolver(url.hostname, { all: true, verbatim: true });
  if (addresses.length === 0) {
    throw new Error(`DAST target has no DNS addresses: ${url.hostname}`);
  }
  for (const { address } of addresses) {
    if (!isPublicAddress(address)) {
      throw new Error(`DAST target DNS includes a non-public address: ${url.hostname}`);
    }
  }
}

export async function verifyRedirectChain(
  initialUrl,
  allowedOrigins,
  forbiddenOrigins,
  { fetchImpl = fetch, resolver = lookup, maxRedirects = 5 } = {},
) {
  let current = initialUrl;
  for (let redirectCount = 0; redirectCount <= maxRedirects; redirectCount += 1) {
    const url = validateDastTarget(current.toString(), allowedOrigins, forbiddenOrigins);
    await verifyPublicResolution(url, resolver);
    const response = await fetchImpl(url, {
      method: 'HEAD',
      redirect: 'manual',
      signal: AbortSignal.timeout(15_000),
    });

    if (response.status >= 300 && response.status < 400) {
      const location = response.headers.get('location');
      if (!location) {
        throw new Error(`DAST target returned a redirect without Location: ${url}`);
      }
      current = new URL(location, url);
      continue;
    }
    if (response.status >= 400) {
      throw new Error(`DAST target preflight failed with HTTP ${response.status}: ${url}`);
    }
    return url;
  }
  throw new Error(`DAST target exceeded ${maxRedirects} redirects: ${initialUrl}`);
}

export function isPublicAddress(address) {
  try {
    assertPublicNetworkAddress(address);
    return true;
  } catch {
    return false;
  }
}

function isLocalHostname(hostname) {
  const normalized = hostname.toLowerCase();
  return (
    normalized === 'localhost' ||
    normalized.endsWith('.localhost') ||
    normalized.endsWith('.local') ||
    normalized.endsWith('.internal')
  );
}

async function main() {
  const allowedOrigins = parseOriginAllowlist(process.env.DAST_ALLOWED_ORIGINS ?? '');
  const forbiddenOrigins = productionOrigins(process.env.DAST_PRODUCTION_ORIGINS);
  for (const [name, raw] of [
    ['ACCOUNT_URL', process.env.ACCOUNT_URL],
    ['API_URL', process.env.API_URL],
    ['BACKOFFICE_URL', process.env.BACKOFFICE_URL],
    ['OPENAPI_URL', process.env.OPENAPI_URL],
  ]) {
    if (!raw) {
      throw new Error(`${name} is required`);
    }
    const target = validateDastTarget(raw, allowedOrigins, forbiddenOrigins);
    await verifyRedirectChain(target, allowedOrigins, forbiddenOrigins);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
