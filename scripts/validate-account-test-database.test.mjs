import assert from 'node:assert/strict';
import test from 'node:test';

import { validateAccountTestDatabaseTarget } from './validate-account-test-database.mjs';

test('accepts only a loopback Account test database', () => {
  assert.deepEqual(
    validateAccountTestDatabaseTarget({
      DATABASE_URL: 'postgres://postgres:test@127.0.0.1:5432/nvbes_account_test',
    }),
    { databaseName: 'nvbes_account_test', hostname: '127.0.0.1' },
  );
});

test('rejects a remote PostgreSQL host', () => {
  assert.throws(
    () =>
      validateAccountTestDatabaseTarget({
        DATABASE_URL: 'postgres://postgres:test@db.example.com/nvbes_account_test',
      }),
    /refuse PostgreSQL host/u,
  );
});

test('rejects a non-test database name', () => {
  assert.throws(
    () =>
      validateAccountTestDatabaseTarget({
        DATABASE_URL: 'postgres://postgres:test@localhost/nvbes_account',
      }),
    /must contain account_test/u,
  );
});

test('accepts host.docker.internal in GitHub Actions CI environment', () => {
  assert.deepEqual(
    validateAccountTestDatabaseTarget({
      DATABASE_URL: 'postgres://postgres:test@host.docker.internal:5432/nvbes_account_test',
      GITHUB_ACTIONS: 'true',
      NVBES_ENV: 'ci',
    }),
    { databaseName: 'nvbes_account_test', hostname: 'host.docker.internal' },
  );
});

test('accepts postgres host in devcontainer environment', () => {
  assert.deepEqual(
    validateAccountTestDatabaseTarget({
      DATABASE_URL: 'postgres://postgres:test@postgres:5432/nvbes_account_test',
      NVBES_DEVCONTAINER: 'true',
    }),
    { databaseName: 'nvbes_account_test', hostname: 'postgres' },
  );
});
