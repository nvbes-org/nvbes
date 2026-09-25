import assert from 'node:assert/strict';
import test from 'node:test';
import {
  evaluateHumanReviewGate,
  parseCodeownersLogins,
  protectedPathsInDiff,
} from './check-human-review-gate.mjs';

test('parseCodeownersLogins extracts user owners', () => {
  const logins = parseCodeownersLogins('* @shaynlink\n/tools/ci/ @alice @org/team\n');
  assert.deepEqual(logins.sort(), ['alice', 'shaynlink']);
});

test('protectedPathsInDiff filters gate files', () => {
  assert.deepEqual(protectedPathsInDiff(['apps/x.rs', 'lefthook.yml']), ['lefthook.yml']);
});

test('evaluateHumanReviewGate waits for CODEOWNER approval', () => {
  const waiting = evaluateHumanReviewGate({
    protectedPaths: ['lefthook.yml'],
    reviews: [{ user: 'dependabot[bot]', state: 'APPROVED' }],
    humanReviewers: ['shaynlink'],
  });
  assert.equal(waiting.ok, false);

  const approved = evaluateHumanReviewGate({
    protectedPaths: ['lefthook.yml'],
    reviews: [{ user: 'shaynlink', state: 'APPROVED' }],
    humanReviewers: ['shaynlink'],
  });
  assert.equal(approved.ok, true);
  assert.match(approved.reason, /approved-by:shaynlink/);
});

test('label human-gate-approved bypasses review wait', () => {
  const result = evaluateHumanReviewGate({
    protectedPaths: ['deny.toml'],
    reviews: [],
    labels: ['human-gate-approved'],
    humanReviewers: ['shaynlink'],
  });
  assert.equal(result.ok, true);
});

test('no protected paths skips human gate', () => {
  assert.equal(
    evaluateHumanReviewGate({
      protectedPaths: [],
      reviews: [],
      humanReviewers: ['shaynlink'],
    }).ok,
    true,
  );
});
