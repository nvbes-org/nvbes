import assert from 'node:assert/strict';
import test from 'node:test';

import { validateIdentityTestDatabaseTarget } from './validate-identity-test-database.mjs';

test('accepts only a loopback identity test database', () => {
  assert.deepEqual(
    validateIdentityTestDatabaseTarget({
      DATABASE_URL: 'postgres://postgres:test@127.0.0.1:5432/nvbes_identity_test',
    }),
    { databaseName: 'nvbes_identity_test', hostname: '127.0.0.1' },
  );
});

test('rejects a remote PostgreSQL host', () => {
  assert.throws(
    () =>
      validateIdentityTestDatabaseTarget({
        DATABASE_URL: 'postgres://postgres:test@db.example.com/nvbes_identity_test',
      }),
    /refuse PostgreSQL host/u,
  );
});

test('rejects a non-test database name', () => {
  assert.throws(
    () =>
      validateIdentityTestDatabaseTarget({
        DATABASE_URL: 'postgres://postgres:test@localhost/nvbes_identity',
      }),
    /must contain identity_test/u,
  );
});
