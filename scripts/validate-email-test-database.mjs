import { isIP } from 'node:net';
import { pathToFileURL } from 'node:url';

const ALLOWED_ENVIRONMENTS = new Set([
  'ci',
  'dev',
  'development',
  'integration',
  'local',
  'test',
  'testing',
]);
const ALLOWED_DATABASES = new Set(['nvbes_email', 'nvbes_email_test']);

function reject(message) {
  throw new Error(message);
}

export function validateEmailTestDatabaseTarget(environment) {
  const targetEnvironment = (environment.NVBES_ENVIRONMENT ?? environment.NVBES_ENV)
    ?.trim()
    .toLowerCase();
  if (targetEnvironment && !ALLOWED_ENVIRONMENTS.has(targetEnvironment)) {
    reject(`email database tests reject environment=${targetEnvironment}`);
  }

  let target;
  try {
    target = new URL(environment.DATABASE_URL);
  } catch {
    reject('DATABASE_URL must be a valid PostgreSQL URL');
  }
  if (!new Set(['postgres:', 'postgresql:']).has(target.protocol)) {
    reject('email database tests require a PostgreSQL URL');
  }

  const hostname = target.hostname.replace(/^\[|\]$/gu, '').toLowerCase();
  const loopback =
    hostname === 'localhost' ||
    (isIP(hostname) === 4 && hostname.split('.')[0] === '127') ||
    (isIP(hostname) === 6 && hostname === '::1');
  const devcontainerPostgres = environment.NVBES_DEVCONTAINER === 'true' && hostname === 'postgres';
  const githubActionsPostgres =
    environment.GITHUB_ACTIONS === 'true' &&
    targetEnvironment === 'ci' &&
    hostname === 'host.docker.internal';
  if (!loopback && !devcontainerPostgres && !githubActionsPostgres) {
    reject(`email database tests refuse PostgreSQL host ${hostname}`);
  }

  const databaseName = decodeURIComponent(target.pathname.replace(/^\/+/, ''));
  if (!ALLOWED_DATABASES.has(databaseName.toLowerCase())) {
    reject('email database tests require the exact nvbes_email or nvbes_email_test database');
  }

  return { databaseName, hostname };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    validateEmailTestDatabaseTarget(process.env);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'invalid email test database target'}\n`,
    );
    process.exitCode = 2;
  }
}
