import { isIP } from 'node:net';
import { pathToFileURL } from 'node:url';
import { isCurrentCiTestDatabaseHost } from './lib/ci-test-database-host.mjs';

function reject(message) {
  throw new Error(message);
}

export function validateAccountTestDatabaseTarget(environment) {
  const targetEnvironment = (environment.NVBES_ENVIRONMENT ?? environment.NVBES_ENV)
    ?.trim()
    .toLowerCase();

  let target;
  try {
    target = new URL(environment.DATABASE_URL);
  } catch {
    reject('DATABASE_URL must be a valid PostgreSQL URL');
  }
  if (!new Set(['postgres:', 'postgresql:']).has(target.protocol)) {
    reject('account database tests require PostgreSQL');
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
  if (
    !loopback &&
    !devcontainerPostgres &&
    !githubActionsPostgres &&
    !isCurrentCiTestDatabaseHost(hostname, environment)
  ) {
    reject(`account database tests refuse PostgreSQL host ${hostname}`);
  }
  const databaseName = decodeURIComponent(target.pathname.replace(/^\/+/, ''));
  if (!databaseName.toLowerCase().includes('account_test')) {
    reject('account database name must contain account_test');
  }
  return { databaseName, hostname };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    validateAccountTestDatabaseTarget(process.env);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'invalid account test database target'}\n`,
    );
    process.exitCode = 2;
  }
}
