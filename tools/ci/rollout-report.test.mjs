import assert from 'node:assert/strict';
import test from 'node:test';
import { summarizeRollout } from './rollout-report.core.mjs';
import { validateCommittedReport, validateRolloutPolicy } from './rollout-policy.mjs';

const evidence = (pr) => ({
  version: 1,
  event: 'pull_request',
  pr,
  head: String(pr),
  packages: ['a'],
  workspace: ['a', 'b'],
  timestamp: `2026-09-${String(pr).padStart(2, '0')}T00:00:00Z`,
  fullSuccess: true,
  scopedSuccess: true,
  divergence: false,
});
test('Rust rollout requires twenty distinct PRs and at least one week', () => {
  const rows = Array.from({ length: 20 }, (_, i) => evidence(i + 1));
  assert.equal(summarizeRollout([{ evidence: rows }]).rustEligibleForReview, true);
  assert.equal(summarizeRollout([{ evidence: rows.slice(0, 19) }]).rustEligibleForReview, false);
  assert.equal(
    summarizeRollout([{ evidence: Array(20).fill(evidence(1)) }]).rustEligibleForReview,
    false,
  );
  assert.equal(
    summarizeRollout([{ evidence: [...rows, { ...evidence(21), divergence: true }] }])
      .rustEligibleForReview,
    false,
  );
});
test('no observations never imply validation or savings', () => {
  const report = summarizeRollout([]);
  assert.equal(report.rustEligibleForReview, false);
  assert.equal(report.comparedPrs, 0);
});
test('affected mode cannot be enabled without reviewed observation evidence', () => {
  const policy = {
    version: 1,
    mode: 'shadow',
    minimumObservationDays: 7,
    minimumRustPullRequests: 20,
    activationEvidence: null,
  };
  assert.doesNotThrow(() => validateRolloutPolicy(policy));
  assert.throws(() => validateRolloutPolicy({ ...policy, mode: 'affected' }));
  assert.throws(() => validateRolloutPolicy({ ...policy, minimumObservationDays: 0 }));
});
test('committed report counters are recomputed from the observations', () => {
  const observations = Array.from({ length: 20 }, (_, index) => evidence(index + 1));
  const report = { ...summarizeRollout([{ evidence: observations }]), observations };
  const policy = { mode: 'affected', activationEvidence: report };
  assert.doesNotThrow(() => validateCommittedReport(policy, report));
  assert.throws(() => validateCommittedReport(policy, { ...report, observations: [] }));
});
