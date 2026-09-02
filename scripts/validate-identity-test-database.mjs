import { isIP } from 'node:net';
import { pathToFileURL } from 'node:url';

function reject(message) {
  throw new Error(message);
}

export function validateIdentityTestDatabaseTarget(environment) {
  let target;
  try {
    target = new URL(environment.DATABASE_URL);
  } catch {
    reject('DATABASE_URL must be a valid PostgreSQL URL');
  }
  if (!new Set(['postgres:', 'postgresql:']).has(target.protocol)) {
    reject('identity database tests require PostgreSQL');
  }
  const hostname = target.hostname.replace(/^\[|\]$/gu, '').toLowerCase();
  const loopback =
    hostname === 'localhost' ||
    (isIP(hostname) === 4 && hostname.split('.')[0] === '127') ||
    (isIP(hostname) === 6 && hostname === '::1');
  if (!loopback) {
    reject(`identity database tests refuse PostgreSQL host ${hostname}`);
  }
  const databaseName = decodeURIComponent(target.pathname.replace(/^\/+/, ''));
  if (!databaseName.toLowerCase().includes('identity_test')) {
    reject('identity database name must contain identity_test');
  }
  return { databaseName, hostname };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    validateIdentityTestDatabaseTarget(process.env);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'invalid identity test database target'}\n`,
    );
    process.exitCode = 2;
  }
}
