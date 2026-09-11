import assert from 'node:assert/strict';
import test from 'node:test';
import { serviceName } from './test-service.mjs';
import { validateAccountTestDatabaseTarget } from '../../scripts/validate-account-test-database.mjs';
import { validateBillingTestDatabaseTarget } from '../../scripts/validate-billing-test-database.mjs';
import { validateEmailTestDatabaseTarget } from '../../scripts/validate-email-test-database.mjs';
import { validateIdentityTestDatabaseTarget } from '../../scripts/validate-identity-test-database.mjs';
import { validateTrustRiskTestDatabaseTarget } from '../../scripts/validate-trust-risk-test-database.mjs';

const ci = {
  GITHUB_ACTIONS: 'true',
  NVBES_ENV: 'ci',
  GITHUB_RUN_ID: '12345',
  GITHUB_RUN_ATTEMPT: '2',
  GITHUB_JOB: 'database',
};
const host = serviceName('postgres', ci);
for (const [domain, validate] of Object.entries({
  account: validateAccountTestDatabaseTarget,
  billing: validateBillingTestDatabaseTarget,
  email: validateEmailTestDatabaseTarget,
  identity: validateIdentityTestDatabaseTarget,
  trust_risk: validateTrustRiskTestDatabaseTarget,
})) {
  const target = (hostname) => `postgres://postgres:test@${hostname}/nvbes_${domain}_test`;
  test(`${domain}: accepts only this CI run, attempt and job service`, () => {
    assert.equal(validate({ ...ci, DATABASE_URL: target(host) }).hostname, host);
    for (const hostname of [
      'nvbes-ci-postgres-999-2-database',
      'nvbes-ci-postgres-12345-1-database',
      'nvbes-ci-postgres-12345-2-other',
      `${host}.example.com`,
      'postgres.example.com',
    ]) {
      assert.throws(() => validate({ ...ci, DATABASE_URL: target(hostname) }));
    }
  });
  test(`${domain}: CI hostname never bypasses environment or database guards`, () => {
    for (const overrides of [
      { GITHUB_ACTIONS: 'false' },
      { NVBES_ENV: 'production' },
      { NVBES_ENVIRONMENT: 'production' },
      { GITHUB_RUN_ID: undefined },
      { GITHUB_RUN_ATTEMPT: undefined },
      { GITHUB_JOB: undefined },
    ]) {
      assert.throws(() => validate({ ...ci, ...overrides, DATABASE_URL: target(host) }));
    }
    assert.throws(() => validate({ ...ci, DATABASE_URL: `postgres://${host}/production` }));
  });
}
