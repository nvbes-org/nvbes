import { createHash, generateKeyPairSync, sign, verify } from 'node:crypto';
import { lstat, mkdir, readFile, realpath, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  REPORT_MAX_AGE_MS,
  REQUIRED_HUMAN_CATEGORIES,
  canonicalTimestamp,
} from './verify-acceptance-evidence.mjs';

const MANIFEST_NAME = 'acceptance-evidence.json';
const MANIFEST_SIGNATURE_NAME = 'acceptance-evidence.sig';
const MAX_MANIFEST_BYTES = 1024 * 1024;
const MAX_REPORT_BYTES = 100 * 1024 * 1024;
const MAX_SIGNATURE_BYTES = 4096;
const MAX_KEY_BYTES = 16_384;
const MAX_VALIDITY_MS = 31 * 24 * 60 * 60 * 1000;
const DEFAULT_VALIDITY_MS = 30 * 24 * 60 * 60 * 1000;
const CLOCK_SKEW_MS = 5 * 60 * 1000;
const RELEASE_PATTERN = /^[a-f0-9]{40,64}$/u;
const SHA256_PATTERN = /^[a-f0-9]{64}$/u;
const SIGNER_KEY_ID_PATTERN = /^[a-z0-9][a-z0-9._-]{2,127}$/u;

export async function signAcceptanceEvidence({
  deploymentKeyId,
  deploymentPath = 'deployment.json',
  deploymentSignaturePath = 'deployment.sig',
  environment = 'staging',
  evidenceRoot,
  expiresAt,
  issuedAt,
  now = new Date(),
  privateKeyPath,
  publicKeyPath,
  release,
  reports,
  signerKeyId,
}) {
  assert(environment === 'staging', 'acceptance evidence must target staging');
  assertPattern(release, RELEASE_PATTERN, 'release is invalid');
  assertPattern(signerKeyId, SIGNER_KEY_ID_PATTERN, 'signer key id is invalid');
  assertPattern(deploymentKeyId, SIGNER_KEY_ID_PATTERN, 'deployment signer key id is invalid');

  const root = await requireCanonicalDirectory(evidenceRoot, 'evidence root');
  const privateKey = await readBoundedFile(
    privateKeyPath,
    MAX_KEY_BYTES,
    'acceptance private key',
    { requireMode0600: true },
  );

  const nowMs = Date.parse(now.toISOString());
  const issuedAtMs = issuedAt ? canonicalTimestamp(issuedAt, 'issuedAt') : nowMs;
  const expiresAtMs = expiresAt
    ? canonicalTimestamp(expiresAt, 'expiresAt')
    : issuedAtMs + DEFAULT_VALIDITY_MS;
  assert(issuedAtMs <= nowMs + CLOCK_SKEW_MS, 'issuedAt is future-dated');
  assert(expiresAtMs > nowMs, 'expiresAt is already elapsed');
  assert(expiresAtMs > issuedAtMs, 'expiry precedes issuance');
  assert(
    expiresAtMs - issuedAtMs <= MAX_VALIDITY_MS,
    'acceptance evidence validity exceeds 31 days',
  );

  const reportEntries = await buildReportEntries(
    reports,
    root,
    release,
    environment,
    issuedAtMs,
    nowMs,
  );
  const deployment = await buildDeploymentEntry({
    deploymentKeyId,
    deploymentPath,
    deploymentSignaturePath,
    root,
  });

  const manifest = {
    deployment,
    environment,
    expiresAt: isoUtc(expiresAtMs),
    issuedAt: isoUtc(issuedAtMs),
    release,
    reports: reportEntries,
    schemaVersion: 1,
    signerKeyId,
  };
  const content = Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  const manifestPath = path.join(root, MANIFEST_NAME);
  const signaturePath = path.join(root, MANIFEST_SIGNATURE_NAME);
  await writeFile(manifestPath, content, { mode: 0o600 });
  const signature = sign(null, content, privateKey);
  await writeFile(signaturePath, signature, { mode: 0o600 });

  let publicKeySha256;
  if (publicKeyPath) {
    const publicKey = await readBoundedFile(publicKeyPath, MAX_KEY_BYTES, 'public key');
    assert(verify(null, content, publicKey, signature), 'round-trip signature verification failed');
    publicKeySha256 = sha256Bytes(publicKey);
  }

  return {
    manifestPath,
    release,
    publicKeySha256,
    signaturePath,
    signerKeyId,
  };
}

export async function generateAcceptanceKey({ directory, name = 'account-acceptance' }) {
  assert(
    typeof name === 'string' && /^[a-z0-9][a-z0-9._-]{0,63}$/u.test(name),
    'key name is invalid',
  );
  await mkdir(directory, { recursive: true, mode: 0o700 });
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const privateFilename = path.join(directory, `${name}-private.pem`);
  const publicFilename = path.join(directory, `${name}-public.pem`);
  await writeFile(privateFilename, privateKey.export({ format: 'pem', type: 'pkcs8' }), {
    mode: 0o600,
    flag: 'wx',
  });
  await writeFile(publicFilename, publicKey.export({ format: 'pem', type: 'spki' }), {
    mode: 0o644,
    flag: 'wx',
  });
  return {
    privateFile: privateFilename,
    publicFile: publicFilename,
    publicKeySha256: createHash('sha256')
      .update(publicKey.export({ format: 'pem', type: 'spki' }))
      .digest('hex'),
  };
}

async function buildReportEntries(reports, root, release, environment, issuedAtMs, nowMs) {
  assert(Array.isArray(reports), 'reports must be an array');
  const byCategory = new Map();
  for (const report of reports) {
    assert(isRecord(report), 'report must be an object');
    assert(
      REQUIRED_HUMAN_CATEGORIES.includes(report.category),
      `unexpected report category ${String(report.category)}`,
    );
    assert(!byCategory.has(report.category), `duplicate report ${report.category}`);
    byCategory.set(report.category, report);
  }
  assert(
    byCategory.size === REQUIRED_HUMAN_CATEGORIES.length,
    'every required human report must be provided exactly once',
  );

  const entries = [];
  for (const category of REQUIRED_HUMAN_CATEGORIES) {
    const report = byCategory.get(category);
    const relativePath = safeRelativePath(report.path ?? `reports/${category}.txt`);
    const content = await readBoundedFile(
      path.join(root, relativePath),
      MAX_REPORT_BYTES,
      `${category} report`,
      { root },
    );
    const signedAtMs = report.signedAt
      ? canonicalTimestamp(report.signedAt, `${category}.signedAt`)
      : issuedAtMs;
    assert(signedAtMs <= issuedAtMs, `${category} was signed after issuance`);
    assert(signedAtMs <= nowMs + CLOCK_SKEW_MS, `${category} is future-dated`);
    assert(
      issuedAtMs - signedAtMs <= REPORT_MAX_AGE_MS[category],
      `${category} report is older than its freshness policy`,
    );
    assert(
      typeof report.signedBy === 'string' && report.signedBy.trim().length > 0,
      `${category} must name its human signer`,
    );
    const entry = {
      category,
      decision: 'passed',
      environment,
      path: relativePath,
      release,
      sha256: sha256Bytes(content),
      signedAt: isoUtc(signedAtMs),
      signedBy: report.signedBy,
    };
    if (category === 'penetration') {
      entry.independent = true;
    }
    entries.push(entry);
  }
  return entries;
}

async function buildDeploymentEntry({
  deploymentKeyId,
  deploymentPath,
  deploymentSignaturePath,
  root,
}) {
  const attestationRelativePath = safeRelativePath(deploymentPath);
  const signatureRelativePath = safeRelativePath(deploymentSignaturePath);
  const attestation = await readBoundedFile(
    path.join(root, attestationRelativePath),
    MAX_REPORT_BYTES,
    'deployment attestation',
    { root },
  );
  await readBoundedFile(
    path.join(root, signatureRelativePath),
    MAX_SIGNATURE_BYTES,
    'deployment attestation signature',
    { root },
  );
  return {
    path: attestationRelativePath,
    sha256: sha256Bytes(attestation),
    signaturePath: signatureRelativePath,
    signerKeyId: deploymentKeyId,
  };
}

async function requireCanonicalDirectory(directoryPath, label) {
  assert(nonEmpty(directoryPath), `${label} path is required`);
  const resolved = path.resolve(directoryPath);
  let metadata;
  try {
    metadata = await lstat(resolved);
  } catch (error) {
    if (error?.code === 'ENOENT') {
      throw new Error(`${label} must be a directory`);
    }
    throw error;
  }
  assert(metadata.isDirectory() && !metadata.isSymbolicLink(), `${label} must be a directory`);
  return realpath(resolved);
}

async function readBoundedFile(filePath, maxBytes, label, options = {}) {
  assert(nonEmpty(filePath), `${label} path is required`);
  const resolved = path.resolve(filePath);
  if (options.root) {
    assert(
      resolved.startsWith(`${options.root}${path.sep}`),
      `${label} path escapes the evidence root`,
    );
  }
  let metadata;
  try {
    metadata = await lstat(resolved);
  } catch (error) {
    if (error?.code === 'ENOENT') {
      throw new Error(`${label} must be a regular file`);
    }
    throw error;
  }
  assert(metadata.isFile() && !metadata.isSymbolicLink(), `${label} must be a regular file`);
  if (options.requireMode0600) {
    assert((metadata.mode & 0o777) === 0o600, `${label} must have permissions 0600`);
  }
  assert(metadata.size > 0 && metadata.size <= maxBytes, `${label} has an invalid size`);
  return readFile(resolved);
}

function safeRelativePath(candidate) {
  assert(nonEmpty(candidate), 'path is required');
  assert(
    candidate === candidate.trim() &&
      !candidate.includes('\\') &&
      ![...candidate].some(
        (character) => character.codePointAt(0) <= 0x1f || character.codePointAt(0) === 0x7f,
      ),
    'path contains invalid characters',
  );
  const normalized = path.posix.normalize(candidate);
  assert(
    normalized === candidate &&
      !path.posix.isAbsolute(candidate) &&
      !candidate.startsWith('../') &&
      candidate !== '..' &&
      candidate.split('/').length <= 8 &&
      candidate.length <= 512,
    'path escapes the evidence root',
  );
  return normalized;
}

function sha256Bytes(content) {
  return createHash('sha256').update(content).digest('hex');
}

function isoUtc(milliseconds) {
  return new Date(milliseconds).toISOString();
}

function assertPattern(value, pattern, message) {
  assert(typeof value === 'string' && pattern.test(value), message);
}

function nonEmpty(value) {
  return typeof value === 'string' && value.trim().length > 0 && value.length <= 256;
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function parseFlags(argv) {
  const flags = new Map();
  const repeated = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (!argument.startsWith('--')) {
      throw new Error(`unexpected argument ${argument}`);
    }
    const separator = argument.indexOf('=');
    const name = separator === -1 ? argument.slice(2) : argument.slice(2, separator);
    const value = separator === -1 ? argv[index + 1] : argument.slice(separator + 1);
    if (separator === -1) {
      index += 1;
    }
    if (name === 'signed-by' || name === 'report-path') {
      repeated.has(name) ? repeated.get(name).push(value) : repeated.set(name, [value]);
    } else {
      flags.set(name, value);
    }
  }
  return { flags, repeated };
}

function collectSignedBy(assignments) {
  const signerByCategory = new Map();
  let defaultSigner;
  for (const assignment of assignments ?? []) {
    const separator = assignment.lastIndexOf('@');
    if (separator === -1) {
      defaultSigner = assignment;
    } else {
      const role = assignment.slice(0, separator);
      const category = assignment.slice(separator + 1);
      assert(REQUIRED_HUMAN_CATEGORIES.includes(category), `unknown category ${category}`);
      signerByCategory.set(category, role);
    }
  }
  return REQUIRED_HUMAN_CATEGORIES.map((category) => ({
    category,
    signedBy: signerByCategory.get(category) ?? defaultSigner,
  }));
}

function collectReportPaths(assignments) {
  const pathByCategory = new Map();
  for (const assignment of assignments ?? []) {
    const separator = assignment.lastIndexOf('@');
    if (separator === -1) {
      throw new Error('report-path must use the form <path>@<category>');
    }
    const filePath = assignment.slice(0, separator);
    const category = assignment.slice(separator + 1);
    assert(REQUIRED_HUMAN_CATEGORIES.includes(category), `unknown category ${category}`);
    pathByCategory.set(category, filePath);
  }
  return REQUIRED_HUMAN_CATEGORIES.map((category) => ({
    category,
    ...(pathByCategory.has(category) ? { path: pathByCategory.get(category) } : {}),
  }));
}

async function main() {
  const args = process.argv.slice(2);
  if (args[0] === 'generate-key') {
    const directory = args[1];
    if (!directory) {
      throw new Error(
        'usage: sign-acceptance-evidence.mjs generate-key <directory> [--name <name>]',
      );
    }
    const { flags } = parseFlags(args.slice(2));
    const result = await generateAcceptanceKey({
      directory,
      name: flags.get('name') ?? 'account-acceptance',
    });
    process.stdout.write(
      `Generated Ed25519 acceptance key pair.\n` +
        `  private: ${result.privateFile} (0600, keep outside the repository)\n` +
        `  public:  ${result.publicFile}\n` +
        `  public key SHA-256: ${result.publicKeySha256}\n`,
    );
    return;
  }

  const { flags, repeated } = parseFlags(args);
  const environment = {
    ACCOUNT_ACCEPTANCE_EVIDENCE_ROOT: 'evidence-root',
    ACCOUNT_ACCEPTANCE_PRIVATE_KEY_FILE: 'private-key',
    NVBES_RELEASE_SHA: 'release',
    ACCOUNT_ACCEPTANCE_TRUSTED_KEY_ID: 'signer-key-id',
    ACCOUNT_DEPLOYMENT_TRUSTED_KEY_ID: 'deployment-key-id',
    ACCOUNT_ACCEPTANCE_ISSUED_AT: 'issued-at',
    ACCOUNT_ACCEPTANCE_EXPIRES_AT: 'expires-at',
    ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_FILE: 'public-key',
  };
  function value(name) {
    return flags.get(name) ?? process.env[environment[name] ?? ''];
  }

  const reports = collectSignedBy(repeated.get('signed-by'));
  const pathOverrides = collectReportPaths(repeated.get('report-path'));
  for (const category of REQUIRED_HUMAN_CATEGORIES) {
    if (pathOverrides.some((entry) => entry.category === category && entry.path)) {
      const override = pathOverrides.find((entry) => entry.category === category && entry.path);
      reports.find((entry) => entry.category === category).path = override.path;
    }
  }

  const result = await signAcceptanceEvidence({
    deploymentKeyId: value('deployment-key-id'),
    evidenceRoot: value('evidence-root'),
    expiresAt: value('expires-at'),
    issuedAt: value('issued-at'),
    privateKeyPath: value('private-key'),
    publicKeyPath: value('public-key'),
    release: value('release'),
    reports,
    signerKeyId: value('signer-key-id'),
  });
  process.stdout.write(
    `Signed Account acceptance evidence for ${result.release} (${result.signerKeyId}).\n` +
      `  manifest:  ${result.manifestPath}\n` +
      `  signature: ${result.signaturePath}\n` +
      (result.publicKeySha256
        ? `  public key SHA-256: ${result.publicKeySha256}\n`
        : `  (add --public-key to print the trust pin and self-verify)\n`),
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'acceptance signing failed'}\n`,
    );
    process.exitCode = 1;
  });
}
