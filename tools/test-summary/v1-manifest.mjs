import assert from 'node:assert/strict';

export const DOMAINS = ['identity', 'account', 'billing', 'email', 'trust-risk', 'platform'];
export const METRICS = ['lines', 'branches', 'mutation'];
export const STATUSES = ['passed', 'failed', 'blocked', 'not-run', 'not-applicable'];
export const nonEmpty = (value) => typeof value === 'string' && value.trim().length > 0;
export const unique = (values) => new Set(values).size === values.length;

export function validateManifest(root, domains) {
  assert.equal(root.schemaVersion, 1, 'Unsupported V1 manifest');
  assert.deepEqual(
    Object.keys(root.domains).sort((a, b) => a.localeCompare(b)),
    [...DOMAINS].sort((a, b) => a.localeCompare(b)),
    'Six domains required',
  );
  assert.equal(domains.length, DOMAINS.length, 'Six domain manifests required');
  assert(unique(domains.map((domain) => domain.domain)), 'Duplicate domain');
  assert.equal(root.policy.silentSkipsAllowed, false);
  assert.equal(root.policy.retryCanMakeGateGreen, false);
  assert.equal(root.policy.testDataMustBeSynthetic, true);
  assert.equal(root.policy.productionLoadTestingAllowed, false);
  for (const metric of METRICS) {
    assert(
      Number.isFinite(root.thresholds[metric]) &&
        root.thresholds[metric] >= 90 &&
        root.thresholds[metric] <= 100,
      `${metric}: minimum 90, maximum 100`,
    );
  }
  const suites = [];
  for (const domain of domains) {
    assert.equal(domain.schemaVersion, 1);
    assert(DOMAINS.includes(domain.domain), 'Unknown domain');
    assert(nonEmpty(domain.project) && nonEmpty(domain.cargoPackage));
    assert(Array.isArray(domain.suites) && domain.suites.length > 0, 'Missing suites');
    assert(Array.isArray(domain.gaps), 'Missing gap registry');
    for (const suite of domain.suites) {
      assert(nonEmpty(suite.id) && nonEmpty(suite.requirement) && nonEmpty(suite.risk));
      assert(['automated', 'human'].includes(suite.execution), 'Invalid execution');
      assert(nonEmpty(suite.target), 'Missing execution target or procedure');
      assert(
        Array.isArray(suite.cases) &&
          suite.cases.length > 0 &&
          suite.cases.every(nonEmpty) &&
          unique(suite.cases),
        'Missing or duplicate test cases',
      );
      suites.push({ ...suite, domain: domain.domain });
    }
    for (const gap of domain.gaps) {
      for (const field of ['id', 'owner', 'risk', 'closure', 'reviewDate']) {
        assert(nonEmpty(gap[field]), `Gap ${gap.id}: missing ${field}`);
      }
      assert(
        /^\d{4}-\d{2}-\d{2}$/u.test(gap.reviewDate) &&
          new Date(gap.reviewDate).toISOString().startsWith(gap.reviewDate),
        'Invalid gap review date',
      );
      assert(['open', 'closed', 'not-applicable'].includes(gap.status));
      if (gap.status !== 'open')
        assert(nonEmpty(gap.evidence), 'Gap disposition requires evidence');
    }
  }
  assert(unique(suites.map((suite) => suite.id)), 'Duplicate suite');
  assert(unique(suites.flatMap((suite) => suite.cases)), 'Duplicate test case');
  return suites;
}
