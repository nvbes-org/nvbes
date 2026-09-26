import assert from 'node:assert/strict';
import test from 'node:test';
import { CRITICAL_PACKAGES, summarizeDiffOutcomes } from './mutation-in-diff.mjs';

test('CRITICAL_PACKAGES covers identity billing account trust-risk', () => {
  assert.equal(CRITICAL_PACKAGES.has('nvbes-identity-service'), true);
  assert.equal(CRITICAL_PACKAGES.has('nvbes-billing'), true);
  assert.equal(CRITICAL_PACKAGES.has('nvbes-email'), false);
});

test('summarizeDiffOutcomes counts missed and critical packages', () => {
  const summary = summarizeDiffOutcomes({
    outcomes: [
      { summary: 'CaughtMutant', scenario: { Mutant: { package: 'nvbes-email', function: 'a' } } },
      {
        summary: 'MissedMutant',
        scenario: { Mutant: { package: 'nvbes-core', function: 'hash' } },
      },
      { summary: 'Timeout', scenario: { Mutant: { package: 'nvbes-billing', function: 'pay' } } },
      {
        summary: 'MissedMutant',
        scenario: { Mutant: { package: 'nvbes-email', function: 'render' } },
      },
    ],
  });
  assert.equal(summary.caught, 1);
  assert.equal(summary.missed, 2);
  assert.equal(summary.timeout, 1);
  assert.equal(summary.tested, 4);
  assert.equal(summary.criticalMissed.length, 2);
  assert.match(summary.criticalMissed[0], /nvbes-core/);
});
