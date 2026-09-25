import assert from 'node:assert/strict';
import test from 'node:test';
import {
  AGENT_AUTHOR_PATTERN,
  HUMAN_REVIEW_TRAILER,
  evaluateLlmCommitPolicy,
  hasAiAssistedTrailer,
  hasHumanReviewRequiredTrailer,
  isAgentCommit,
  isProtectedPath,
  isValidAiAssistedTrailer,
} from './check-agent-protected-paths.mjs';

test('isProtectedPath covers gates and thresholds', () => {
  assert.equal(isProtectedPath('lefthook.yml'), true);
  assert.equal(isProtectedPath('tools/ci/foo.mjs'), true);
  assert.equal(isProtectedPath('AGENTS.md'), true);
  assert.equal(isProtectedPath('apps/identity-service/src/main.rs'), false);
});

test('AI-Assisted and Human-Review trailers', () => {
  assert.equal(hasAiAssistedTrailer('feat: x\n\nAI-Assisted: Cursor\n'), true);
  assert.equal(isValidAiAssistedTrailer('feat: x\n\nAI-Assisted:\n'), false);
  assert.equal(hasHumanReviewRequiredTrailer(`x\n\n${HUMAN_REVIEW_TRAILER}\n`), true);
  assert.equal(hasHumanReviewRequiredTrailer('x\n\nHuman-Review-Required: other\n'), false);
});

test('agent identity from trailer or author', () => {
  assert.equal(isAgentCommit({ message: 'x\n\nAI-Assisted: Cursor\n' }), true);
  assert.equal(isAgentCommit({ authorName: 'Cursor Agent' }), true);
  assert.equal(isAgentCommit({ authorName: 'shaynlink', message: 'feat: x' }), false);
  assert.equal(AGENT_AUTHOR_PATTERN.test('Codex'), true);
});

test('LLM may touch protected paths when Human-Review-Required is present', () => {
  const ok = evaluateLlmCommitPolicy(['lefthook.yml', 'apps/x.rs'], {
    isAgent: true,
    hasAiAssisted: true,
    hasHumanReview: true,
  });
  assert.equal(ok.ok, true);
  assert.equal(ok.requiresHumanReview, true);
  assert.deepEqual(ok.protectedTouched, ['lefthook.yml']);
});

test('LLM without Human-Review-Required is blocked on protected paths', () => {
  const blocked = evaluateLlmCommitPolicy(['tools/ci/x.mjs'], {
    isAgent: true,
    hasAiAssisted: true,
    hasHumanReview: false,
  });
  assert.equal(blocked.ok, false);
  assert.match(blocked.reasons[0], /Human-Review-Required/);
});

test('agent author without AI-Assisted is blocked', () => {
  const blocked = evaluateLlmCommitPolicy(['apps/x.rs'], {
    isAgent: true,
    hasAiAssisted: false,
    hasHumanReview: false,
  });
  assert.equal(blocked.ok, false);
  assert.match(blocked.reasons[0], /AI-Assisted/);
});
