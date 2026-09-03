import assert from 'node:assert/strict';
import test from 'node:test';

import { validateBillingTestDatabaseTarget } from './validate-billing-test-database.mjs';

test('accepts only a loopback Billing test database', () => {
  assert.deepEqual(
    validateBillingTestDatabaseTarget({
      DATABASE_URL: 'postgres://postgres:test@127.0.0.1:5432/nvbes_billing_test',
    }),
    { databaseName: 'nvbes_billing_test', hostname: '127.0.0.1' },
  );
});

test('rejects a remote PostgreSQL host', () => {
  assert.throws(
    () =>
      validateBillingTestDatabaseTarget({
        DATABASE_URL: 'postgres://postgres:test@db.example.com/nvbes_billing_test',
      }),
    /refuse PostgreSQL host/u,
  );
});

test('rejects a non-test database name', () => {
  assert.throws(
    () =>
      validateBillingTestDatabaseTarget({
        DATABASE_URL: 'postgres://postgres:test@localhost/nvbes_billing',
      }),
    /must contain billing_test/u,
  );
});
