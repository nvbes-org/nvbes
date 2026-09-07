import assert from 'node:assert/strict';
import test from 'node:test';
import { summarizeRollout } from './rollout-report.core.mjs';
import { validateCommittedReport, validateRolloutPolicy } from './rollout-policy.mjs';
import { runMeasurements, scopeMetric } from './rollout-metrics.mjs';
import { collectRunLogs } from './rollout-logs.mjs';

test('cancelled jobs without steps do not hide executed job logs', () => {
  const jobs = [
    { id: 10, conclusion: 'success', steps: [{ name: 'test' }] },
    { id: 11, conclusion: 'cancelled', steps: [] },
  ];
  const missing = Object.assign(new Error('missing log'), { stderr: 'log not found: 11' });
  const gh = (...args) => {
    if (!args.includes('--job')) throw missing;
    assert.equal(args[args.indexOf('--job') + 1], '10');
    return 'executed job evidence';
  };
  assert.equal(collectRunLogs({ id: 1 }, jobs, 'owner/repo', gh), 'executed job evidence');
  assert.throws(
    () =>
      collectRunLogs(
        { id: 1 },
        [...jobs.slice(0, 1), { ...jobs[1], steps: [{ name: 'test' }] }],
        'owner/repo',
        gh,
      ),
    /missing log/u,
  );
  assert.throws(
    () =>
      collectRunLogs({ id: 1 }, jobs, 'owner/repo', () => {
        throw Object.assign(new Error('expired execution log'), { stderr: 'log not found: 10' });
      }),
    /expired execution log/u,
  );
});

test('measurements separate elapsed time from rounded runner minutes', () => {
  const job = (name, start, end, conclusion = 'success') => ({
    name,
    started_at: `2026-09-07T00:00:${start}Z`,
    completed_at: `2026-09-07T00:00:${end}Z`,
    conclusion,
  });
  const [measurement] = runMeasurements([
    {
      id: 1,
      scopes: [{ category: 'ci', shadow: false, planningMs: 12 }],
      jobs: [
        job('scope', '00', '10'),
        job('contracts', '10', '40'),
        job('other', '10', '35'),
        job('rust', '00', '59', 'skipped'),
      ],
    },
  ]);
  assert.equal(measurement.elapsedSeconds, 40);
  assert.equal(measurement.runnerMinutes, 3);
  assert.equal(measurement.category, 'ci');
  assert.equal(measurement.planningMs, 12);
  assert.equal(runMeasurements([{}])[0].planningMs, null);
  assert.equal(runMeasurements([{}])[0].category, 'unclassified');
});

test('scope categories distinguish fast paths, fallbacks and runtime combinations', () => {
  const plan = { candidate: { contracts: true, rust: true, database: true }, shadow: true };
  assert.equal(scopeMetric(plan, Date.now()).category, 'database+rust');
  assert.equal(scopeMetric({ ...plan, docsOnly: true }, Date.now()).category, 'docs');
  assert.equal(scopeMetric({ ...plan, ciOnly: true }, Date.now()).category, 'ci');
  assert.equal(scopeMetric({ ...plan, fallbackFull: true }, Date.now()).category, 'fallback-full');
});

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
