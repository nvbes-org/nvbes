import { assertPublicNetworkAddress, ipAddressFamily } from './target-address-policy.js';

const LOCAL_ENVIRONMENTS = new Set(['ci', 'development', 'local', 'test']);
const LOCAL_SERVICE_HOSTS = new Set(['127.0.0.1', '::1', 'host.docker.internal', 'localhost']);
const LOCAL_WEB_HOSTS = new Set(['127.0.0.1', '::1', 'localhost']);
const KNOWN_PRODUCTION_ORIGINS = new Set([
  'https://account.nvbes.fr',
  'https://api.nvbes.fr',
  'https://backoffice.nvbes.fr',
  'https://cloud.nvbes.fr',
]);

export function validateTargetOrigin({
  allowedOrigins = '',
  kind = 'service',
  productionOrigins = '',
  target,
  targetEnvironment,
}) {
  const parsed = parseStrictOrigin(target);
  rejectProduction(parsed, targetEnvironment, productionOrigins);

  if (targetEnvironment === 'staging') {
    if (parsed.protocol !== 'https') {
      throw new Error('Staging quality targets must use HTTPS.');
    }
    rejectInternalHostname(parsed.hostname);

    const allowlist = parseAllowlist(allowedOrigins, productionOrigins);
    if (!allowlist.has(parsed.origin)) {
      throw new Error(`Target origin is not in the staging allowlist: ${parsed.origin}`);
    }
    return parsed.origin;
  }

  if (!LOCAL_ENVIRONMENTS.has(targetEnvironment)) {
    throw new Error(`Unsupported non-production target environment: ${targetEnvironment}`);
  }

  const allowedHosts = kind === 'web' ? LOCAL_WEB_HOSTS : LOCAL_SERVICE_HOSTS;
  if (parsed.protocol !== 'http' || !allowedHosts.has(parsed.hostname)) {
    throw new Error(`${kind} target must be an allowlisted local HTTP origin.`);
  }

  return parsed.origin;
}

export async function validateResolvedTargetOrigin({
  allowedOrigins = '',
  kind = 'service',
  productionOrigins = '',
  resolveHostname,
  target,
  targetEnvironment,
}) {
  const origin = validateTargetOrigin({
    allowedOrigins,
    kind,
    productionOrigins,
    target,
    targetEnvironment,
  });
  if (LOCAL_ENVIRONMENTS.has(targetEnvironment)) {
    return origin;
  }

  const { hostname } = parseStrictOrigin(target);
  if (ipAddressFamily(hostname) !== 0) {
    assertPublicNetworkAddress(hostname);
    return origin;
  }
  if (typeof resolveHostname !== 'function') {
    throw new Error('A DNS resolver is required for every non-local quality target.');
  }

  let records;
  try {
    records = await resolveHostname(hostname);
  } catch {
    throw new Error('The non-local quality target could not be resolved safely.');
  }
  if (!Array.isArray(records) || records.length === 0) {
    throw new Error('The non-local quality target did not resolve to an address.');
  }
  for (const record of records) {
    const address = typeof record === 'string' ? record : record?.address;
    if (typeof address !== 'string' || ipAddressFamily(address) === 0) {
      throw new Error('The DNS resolver returned an invalid address record.');
    }
    assertPublicNetworkAddress(address);
  }
  return origin;
}

export function parseStrictOrigin(target) {
  if (typeof target !== 'string' || target === '' || target !== target.trim()) {
    throw new Error('Target must be a non-empty origin without surrounding whitespace.');
  }
  if (target.includes('\\') || target.includes('%') || hasControlCharacter(target)) {
    throw new Error('Target contains forbidden control, escape or percent-encoded characters.');
  }

  const match = /^(https?):\/\/(\[[0-9a-f:.]+\]|[a-z0-9.-]+)(?::([0-9]{1,5}))?\/?$/iu.exec(target);
  if (!match) {
    throw new Error(
      'Target must be an HTTP(S) origin without credentials, path, query or fragment.',
    );
  }

  const protocol = match[1].toLowerCase();
  const rawHostname = match[2].toLowerCase();
  const hostname = rawHostname.startsWith('[') ? rawHostname.slice(1, -1) : rawHostname;
  validateHostname(hostname);

  const port = match[3] || '';
  if (port !== '' && (port.startsWith('0') || Number(port) > 65_535)) {
    throw new Error('Target port must be canonical and between 1 and 65535.');
  }

  const renderedHostname = rawHostname.startsWith('[') ? `[${hostname}]` : hostname;
  return {
    hostname,
    origin: `${protocol}://${renderedHostname}${port === '' ? '' : `:${port}`}`,
    port,
    protocol,
  };
}

function hasControlCharacter(value) {
  for (let index = 0; index < value.length; index += 1) {
    const codeUnit = value.charCodeAt(index);
    if (codeUnit <= 0x20 || codeUnit === 0x7f) {
      return true;
    }
  }
  return false;
}

function parseAllowlist(rawAllowlist, productionOrigins) {
  const values = rawAllowlist
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean);

  if (values.length === 0) {
    throw new Error('A non-empty exact staging origin allowlist is required.');
  }

  const origins = new Set();
  for (const value of values) {
    const parsed = parseStrictOrigin(value);
    if (parsed.protocol !== 'https') {
      throw new Error('Every staging allowlist entry must use HTTPS.');
    }
    rejectProduction(parsed, 'staging', productionOrigins);
    rejectInternalHostname(parsed.hostname);
    origins.add(parsed.origin);
  }
  return origins;
}

function rejectProduction(parsed, targetEnvironment, rawProductionOrigins) {
  const deniedOrigins = productionOriginDenylist(rawProductionOrigins, targetEnvironment);
  const productionHostname =
    parsed.hostname.endsWith('.prod.nvbes.fr') ||
    deniedOrigins.hostnames.has(parsed.hostname) ||
    deniedOrigins.origins.has(parsed.origin);
  if (productionHostname || targetEnvironment === 'production') {
    throw new Error('Account quality tests are forbidden against production.');
  }
}

function productionOriginDenylist(rawProductionOrigins, targetEnvironment) {
  const configured =
    targetEnvironment === 'staging'
      ? rawProductionOrigins
          .split(',')
          .map((value) => value.trim())
          .filter(Boolean)
      : [];
  if (targetEnvironment === 'staging' && configured.length === 0) {
    throw new Error('A protected production-origin denylist is required for staging tests.');
  }
  const origins = new Set();
  const hostnames = new Set();
  for (const value of [...KNOWN_PRODUCTION_ORIGINS, ...configured]) {
    const parsed = parseStrictOrigin(value);
    if (parsed.protocol !== 'https') {
      throw new Error('Every production denylist entry must use HTTPS.');
    }
    origins.add(parsed.origin);
    hostnames.add(parsed.hostname);
  }
  return { hostnames, origins };
}

function rejectInternalHostname(hostname) {
  if (
    LOCAL_SERVICE_HOSTS.has(hostname) ||
    hostname === '0.0.0.0' ||
    hostname.endsWith('.internal') ||
    hostname.endsWith('.local') ||
    (ipAddressFamily(hostname) !== 0 && rejectsPublicNetworkPolicy(hostname)) ||
    (ipAddressFamily(hostname) === 0 && /^\d+(?:\.\d+){0,3}$/u.test(hostname))
  ) {
    throw new Error('Staging targets cannot resolve to an explicitly internal hostname.');
  }
}

function rejectsPublicNetworkPolicy(hostname) {
  try {
    assertPublicNetworkAddress(hostname);
    return false;
  } catch {
    return true;
  }
}

function validateHostname(hostname) {
  if (hostname === '::1') {
    return;
  }
  if (hostname.includes(':')) {
    throw new Error('Only the IPv6 loopback address is accepted.');
  }
  if (hostname.endsWith('.') || hostname.length > 253) {
    throw new Error('Target hostname is not canonical.');
  }

  const labels = hostname.split('.');
  if (
    labels.some(
      (label) =>
        label.length === 0 ||
        label.length > 63 ||
        !/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/u.test(label),
    )
  ) {
    throw new Error('Target hostname contains an invalid DNS label.');
  }

  if (/^\d+\.\d+\.\d+\.\d+$/u.test(hostname)) {
    const octets = hostname.split('.').map(Number);
    if (octets.some((octet) => octet > 255)) {
      throw new Error('Target contains an invalid IPv4 address.');
    }
  }
}
