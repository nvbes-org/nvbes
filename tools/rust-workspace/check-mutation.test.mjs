import assert from 'node:assert/strict';
import { test } from 'node:test';
import { aggregateMutationOutcomes, evaluateMutation, mutationScore } from './check-mutation.mjs';

function mutant(packageName, summary) {
  return {
    scenario: {
      Mutant: {
        package: packageName,
        file: `libs/rust/${packageName.replace('nvbes-', '')}/src/lib.rs`,
      },
    },
    summary,
  };
}

function baselineOutcome() {
  return { scenario: null, summary: 'Success' };
}

function examplePackages() {
  return [
    { name: 'nvbes-audit', manifest_path: '/repo/libs/rust/audit/Cargo.toml' },
    { name: 'nvbes-dpop', manifest_path: '/repo/libs/rust/dpop/Cargo.toml' },
    { name: 'nvbes-email', manifest_path: '/repo/libs/rust/email/Cargo.toml' },
    { name: 'nvbes-ports', manifest_path: '/repo/libs/rust/ports/Cargo.toml' },
  ];
}

function exampleConfig() {
  return {
    schemaVersion: 1,
    inclusionFloor: 50,
    crates: {
      'nvbes-audit': { threshold: 70, baseline: { score: 75.0 } },
      'nvbes-dpop': { threshold: 73, baseline: { score: 78.6 } },
      'nvbes-email': { threshold: 79, baseline: { score: 84.8 } },
    },
    excluded: [
      {
        crate: 'nvbes-ports',
        reason: 'below floor',
        baseline: { score: 0.0 },
      },
    ],
  };
}

test('scores caught mutants over tested mutants, ignoring unviable ones', () => {
  const outcomes = [
    baselineOutcome(),
    mutant('nvbes-audit', 'CaughtMutant'),
    mutant('nvbes-audit', 'MissedMutant'),
    mutant('nvbes-audit', 'Unviable'),
  ];
  const stats = aggregateMutationOutcomes(outcomes);
  assert.equal(stats.get('nvbes-audit').tested, 2);
  assert.equal(stats.get('nvbes-audit').unviable, 1);
  assert.equal(mutationScore(stats.get('nvbes-audit')), 50);
});

test('counts a timeout as a missed mutant (not caught)', () => {
  const outcomes = [
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'Timeout'),
  ];
  const stats = aggregateMutationOutcomes(outcomes);
  assert.equal(stats.get('nvbes-email').tested, 3);
  assert.equal(stats.get('nvbes-email').timeout, 1);
  assert.equal(Math.round(mutationScore(stats.get('nvbes-email'))), 67);
});

test('passes when every crate is at or above its mutation threshold', () => {
  const outcomes = [
    baselineOutcome(),
    mutant('nvbes-audit', 'CaughtMutant'),
    mutant('nvbes-audit', 'CaughtMutant'),
    mutant('nvbes-audit', 'CaughtMutant'),
    mutant('nvbes-audit', 'CaughtMutant'),
    mutant('nvbes-audit', 'CaughtMutant'),
    mutant('nvbes-audit', 'CaughtMutant'),
    mutant('nvbes-audit', 'CaughtMutant'),
    mutant('nvbes-audit', 'MissedMutant'),
    mutant('nvbes-dpop', 'CaughtMutant'),
    mutant('nvbes-dpop', 'CaughtMutant'),
    mutant('nvbes-dpop', 'CaughtMutant'),
    mutant('nvbes-dpop', 'CaughtMutant'),
    mutant('nvbes-dpop', 'CaughtMutant'),
    mutant('nvbes-dpop', 'CaughtMutant'),
    mutant('nvbes-dpop', 'MissedMutant'),
    mutant('nvbes-dpop', 'MissedMutant'),
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'CaughtMutant'),
    mutant('nvbes-email', 'MissedMutant'),
  ];
  const result = evaluateMutation(outcomes, exampleConfig(), examplePackages());
  assert.deepEqual(result.failures, []);
  assert.equal(result.rows.length, 3);
});

test('detects a mutation score gap when a crate falls below its threshold', () => {
  const outcomes = [mutant('nvbes-audit', 'CaughtMutant'), mutant('nvbes-audit', 'MissedMutant')];
  const config = {
    schemaVersion: 1,
    crates: { 'nvbes-audit': { threshold: 90, baseline: { score: null } } },
    excluded: [],
  };
  const result = evaluateMutation(outcomes, config, examplePackages());
  assert.equal(Math.round(result.rows[0].score), 50);
  assert.ok(result.failures.some((failure) => failure.startsWith('nvbes-audit')));
  assert.match(
    result.failures.find((failure) => failure.startsWith('nvbes-audit')),
    /50\.0%/u,
  );
});

test('flags a configured crate absent from the outcomes as a gap', () => {
  const outcomes = [];
  const result = evaluateMutation(outcomes, exampleConfig(), examplePackages());
  assert.ok(result.rows.find((row) => row.crate === 'nvbes-email').gap);
  assert.equal(result.rows.find((row) => row.crate === 'nvbes-email').score, 0);
});

test('rejects an unsupported outcome summary', () => {
  const outcomes = [mutant('nvbes-audit', 'Exploded')];
  assert.throws(
    () => evaluateMutation(outcomes, exampleConfig(), examplePackages()),
    /Unsupported cargo-mutants outcome summary/u,
  );
});

test('rejects a crate that is both configured and excluded', () => {
  const config = {
    schemaVersion: 1,
    crates: { 'nvbes-email': { threshold: 79, baseline: { score: 84.8 } } },
    excluded: [{ crate: 'nvbes-email', reason: 'whoops' }],
  };
  assert.throws(
    () => evaluateMutation([], config, examplePackages()),
    /both configured and excluded/u,
  );
});

test('rejects a configured crate that is not a workspace member', () => {
  const config = {
    schemaVersion: 1,
    crates: { 'nvbes-ghost': { threshold: 0, baseline: { score: null } } },
    excluded: [],
  };
  assert.throws(() => evaluateMutation([], config, examplePackages()), /not a workspace member/u);
});

test('rejects an excluded crate without a justification or baseline', () => {
  const config = {
    schemaVersion: 1,
    crates: {},
    excluded: [{ crate: 'nvbes-ports', reason: '', baseline: { score: null } }],
  };
  assert.throws(() => evaluateMutation([], config, examplePackages()), /explicit reason/u);
});
