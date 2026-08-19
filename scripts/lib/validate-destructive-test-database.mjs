import { isIP } from 'node:net';
import { pathToFileURL } from 'node:url';

const OPT_IN = 'account-quality-v1';
const ALLOWED_ENVIRONMENTS = new Set([
  'ci',
  'dev',
  'development',
  'integration',
  'local',
  'test',
  'testing',
]);
const PRODUCTION_SEGMENTS = new Set(['live', 'prod', 'production', 'staging']);

function fail(message) {
  throw new Error(message);
}

export function validateDestructiveTestDatabaseTarget(environment) {
  if (environment.NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE !== OPT_IN) {
    fail(`NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE must equal ${OPT_IN}`);
  }

  const targetEnvironment = environment.NVBES_ENV?.trim().toLowerCase();
  if (targetEnvironment && !ALLOWED_ENVIRONMENTS.has(targetEnvironment)) {
    fail(`destructive Account tests reject NVBES_ENV=${targetEnvironment}`);
  }

  let target;
  try {
    target = new URL(environment.NVBES_DATABASE_URL ?? environment.DATABASE_URL);
  } catch {
    fail('NVBES_DATABASE_URL or DATABASE_URL must be a valid PostgreSQL URL');
  }
  if (!['postgres:', 'postgresql:'].includes(target.protocol)) {
    fail('destructive Account tests require a PostgreSQL URL');
  }

  const hostname = target.hostname.replace(/^\[|\]$/gu, '').toLowerCase();
  const loopback =
    hostname === 'localhost' ||
    (isIP(hostname) === 4 && hostname.split('.')[0] === '127') ||
    (isIP(hostname) === 6 && hostname === '::1');
  if (!loopback) {
    fail(`destructive Account tests require loopback PostgreSQL; refusing ${hostname}`);
  }

  const databaseName = decodeURIComponent(target.pathname.replace(/^\/+/u, ''));
  const segments = databaseName
    .toLowerCase()
    .split(/[^a-z0-9]+/u)
    .filter(Boolean);
  if (
    !segments.includes('test') ||
    segments.some((segment) => PRODUCTION_SEGMENTS.has(segment)) ||
    ['postgres', 'template0', 'template1'].includes(databaseName.toLowerCase())
  ) {
    fail(
      'destructive Account tests require a disposable database with an exact test segment and no production-like segment',
    );
  }

  return { databaseName };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    validateDestructiveTestDatabaseTarget(process.env);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'invalid destructive test database target'}\n`,
    );
    process.exitCode = 2;
  }
}
