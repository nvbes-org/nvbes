import assert from 'node:assert/strict';
import test from 'node:test';

import { validateDestructiveTestDatabaseTarget } from '../../scripts/lib/validate-destructive-test-database.mjs';

function environment(databaseUrl, overrides = {}) {
  return {
    NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE: 'account-quality-v1',
    NVBES_DATABASE_URL: databaseUrl,
    NVBES_ENV: 'test',
    ...overrides,
  };
}

test('destructive database guard accepts only explicit loopback test targets', () => {
  for (const target of [
    'postgres://user:secret@localhost:5432/nvbes_test',
    'postgresql://user:secret@127.42.0.1:5432/account_integration_test',
    'postgres://user:secret@[::1]:5432/test_account',
  ]) {
    assert.doesNotThrow(() =>
      validateDestructiveTestDatabaseTarget(environment(target)),
    );
  }
});

test('destructive database guard rejects ambiguous, remote and production-like targets', () => {
  for (const target of [
    'postgres://user:secret@localhost:5432/latest',
    'postgres://user:secret@localhost:5432/latesttest',
    'postgres://user:secret@localhost:5432/latest_production',
    'postgres://user:secret@localhost:5432/production_test',
    'postgres://user:secret@localhost.evil.test:5432/nvbes_test',
    'postgres://user:secret@10.0.0.5:5432/nvbes_test',
    'https://localhost:5432/nvbes_test',
  ]) {
    assert.throws(() =>
      validateDestructiveTestDatabaseTarget(environment(target)),
    );
  }
});

test('destructive database guard requires the exact opt-in and a safe environment', () => {
  const target = 'postgres://user:secret@localhost:5432/nvbes_test';
  assert.throws(() =>
    validateDestructiveTestDatabaseTarget(
      environment(target, { NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE: undefined }),
    ),
  );
  assert.throws(() =>
    validateDestructiveTestDatabaseTarget(
      environment(target, { NVBES_ENV: 'staging' }),
    ),
  );
});
