import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { profileDuration } from '../load-tests/account/profiles.js';
import { validateTargetOrigin } from '../load-tests/account/target-policy.js';

const rawOutputPath = process.argv[2];
const summaryName = process.argv[3];
if (!rawOutputPath || !summaryName) {
  throw new Error('Usage: node write-load-metadata.mjs <output-path> <summary-name>');
}

const outputPath = resolveOutputPath(rawOutputPath);

const targetOrigin = validateTargetOrigin({
  allowedOrigins: process.env.ACCOUNT_LOAD_ALLOWED_ORIGINS,
  kind: 'service',
  productionOrigins: process.env.ACCOUNT_PRODUCTION_DENIED_ORIGINS,
  target: process.env.ACCOUNT_SERVICE_BASE_URL,
  targetEnvironment: process.env.NVBES_TARGET_ENV,
});
if (!['account-api', 'account-session'].includes(process.env.K6_SUITE)) {
  throw new Error('K6_SUITE must be account-api or account-session.');
}

const metadata = {
  configuredDuration: profileDuration(process.env.K6_PROFILE, process.env.ACCOUNT_K6_DURATION),
  configuredIterations: optionalInteger('ACCOUNT_K6_ITERATIONS'),
  configuredLoadRate: optionalInteger('ACCOUNT_K6_LOAD_RATE'),
  datasetId: optionalLabel('ACCOUNT_K6_DATASET_ID'),
  generatedAt: new Date().toISOString(),
  profile: process.env.K6_PROFILE,
  release: releaseIdentifier(),
  replicas: optionalInteger('ACCOUNT_K6_REPLICAS'),
  schemaVersion: 1,
  suite: process.env.K6_SUITE,
  summaryFile: summaryName,
  targetEnvironment: process.env.NVBES_TARGET_ENV,
  targetOrigin,
};

await mkdir(path.dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(metadata, null, 2)}\n`, {
  encoding: 'utf8',
  mode: 0o600,
});

function optionalInteger(name) {
  const raw = process.env[name];
  if (!raw) {
    return null;
  }
  const parsed = Number(raw);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new Error(`${name} must be a positive integer.`);
  }
  return parsed;
}

function optionalLabel(name) {
  const raw = process.env[name];
  if (!raw) {
    return null;
  }
  if (!/^[a-z0-9][a-z0-9._-]{0,79}$/iu.test(raw)) {
    throw new Error(`${name} must be a short public identifier.`);
  }
  return raw;
}

function resolveOutputPath(rawPath) {
  if (/[\0\r\n]/u.test(rawPath) || rawPath.split(/[\\/]+/u).includes('..')) {
    throw new Error('outputPath must not contain traversal segments or control characters');
  }
  return path.resolve(process.cwd(), rawPath);
}

function releaseIdentifier() {
  const raw = process.env.NVBES_RELEASE || 'local';
  if (process.env.NVBES_TARGET_ENV === 'staging' && !/^[a-f0-9]{7,64}$/u.test(raw)) {
    throw new Error('NVBES_RELEASE must be an immutable hexadecimal revision in staging.');
  }
  if (process.env.NVBES_TARGET_ENV !== 'staging' && !/^[a-z0-9][a-z0-9._-]{0,79}$/iu.test(raw)) {
    throw new Error('NVBES_RELEASE must be a short public identifier.');
  }
  return raw;
}
