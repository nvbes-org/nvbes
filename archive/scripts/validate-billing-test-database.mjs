import { isIP } from 'node:net';
import { pathToFileURL } from 'node:url';
import { isCurrentCiTestDatabaseHost } from './lib/ci-test-database-host.mjs';

function reject(message) {
  throw new Error(message);
}

export function validateBillingTestDatabaseTarget(environment) {
  let target;
  try {
    target = new URL(environment.DATABASE_URL);
  } catch {
    reject('DATABASE_URL must be a valid PostgreSQL URL');
  }
  if (!new Set(['postgres:', 'postgresql:']).has(target.protocol)) {
    reject('billing database tests require PostgreSQL');
  }
  const hostname = target.hostname.replace(/^\[|\]$/gu, '').toLowerCase();
  const loopback =
    hostname === 'localhost' ||
    (isIP(hostname) === 4 && hostname.split('.')[0] === '127') ||
    (isIP(hostname) === 6 && hostname === '::1');
  if (!loopback && !isCurrentCiTestDatabaseHost(hostname, environment)) {
    reject(`billing database tests refuse PostgreSQL host ${hostname}`);
  }
  const databaseName = decodeURIComponent(target.pathname.replace(/^\/+/, ''));
  if (!databaseName.toLowerCase().includes('billing_test')) {
    reject('billing database name must contain billing_test');
  }
  return { databaseName, hostname };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    validateBillingTestDatabaseTarget(process.env);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'invalid billing test database target'}\n`,
    );
    process.exitCode = 2;
  }
}
