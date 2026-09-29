import assert from 'node:assert/strict';
import test from 'node:test';
import {
  crateThreshold,
  evaluateThresholdMonotonicity,
  normalizeThresholdConfig,
} from './check-thresholds-monotone.mjs';

test('crateThreshold reads scalar and lines forms', () => {
  assert.equal(crateThreshold({ a: { threshold: 90 } }, 'a'), 90);
  assert.equal(crateThreshold({ a: { threshold: { lines: 88, functions: 80 } } }, 'a'), 88);
  assert.equal(crateThreshold({}, 'missing'), null);
});

test('evaluateThresholdMonotonicity refuses lowered thresholds', () => {
  const base = {
    crates: {
      'nvbes-core': { threshold: 90 },
      'nvbes-email': { threshold: { lines: 95, functions: 90, regions: 90 } },
    },
    excluded: [],
  };
  const head = {
    crates: {
      'nvbes-core': { threshold: 85 },
      'nvbes-email': { threshold: { lines: 95, functions: 90, regions: 90 } },
    },
    excluded: [],
  };
  const failures = evaluateThresholdMonotonicity(base, head, 'cov');
  assert.equal(failures.length, 1);
  assert.match(failures[0], /nvbes-core/);
});

test('evaluateThresholdMonotonicity refuses gated -> excluded', () => {
  const base = { crates: { 'nvbes-core': { threshold: 90 } }, excluded: [] };
  const head = {
    crates: {},
    excluded: [{ crate: 'nvbes-core', reason: 'noise', baseline: { score: 10 } }],
  };
  const failures = evaluateThresholdMonotonicity(base, head, 'mut');
  assert.equal(failures.length, 1);
  assert.match(failures[0], /excluded/);
});

test('normalizeThresholdConfig tolerates empty input', () => {
  assert.deepEqual(normalizeThresholdConfig(null), { crates: {}, excluded: new Set() });
});
