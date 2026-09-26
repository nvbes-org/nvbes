import assert from 'node:assert/strict';
import test from 'node:test';
import {
  AGENT_AUTHOR_PATTERN,
  evaluateLlmCommitPolicy,
  hasAiAssistedTrailer,
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

test('AI-Assisted trailer format', () => {
  assert.equal(hasAiAssistedTrailer('feat: x\n\nAI-Assisted: Cursor\n'), true);
  assert.equal(isValidAiAssistedTrailer('feat: x\n\nAI-Assisted:\n'), false);
});

test('agent identity from trailer or author', () => {
  assert.equal(isAgentCommit({ message: 'x\n\nAI-Assisted: Cursor\n' }), true);
  assert.equal(isAgentCommit({ authorName: 'Cursor Agent' }), true);
  assert.equal(isAgentCommit({ authorName: 'shaynlink', message: 'feat: x' }), false);
  assert.equal(AGENT_AUTHOR_PATTERN.test('Codex'), true);
});

test('LLM may touch protected paths with AI-Assisted only', () => {
  const ok = evaluateLlmCommitPolicy(['lefthook.yml', 'apps/x.rs'], {
    isAgent: true,
    hasAiAssisted: true,
  });
  assert.equal(ok.ok, true);
  assert.deepEqual(ok.protectedTouched, ['lefthook.yml']);
});

test('agent author without AI-Assisted is blocked', () => {
  const blocked = evaluateLlmCommitPolicy(['apps/x.rs'], {
    isAgent: true,
    hasAiAssisted: false,
  });
  assert.equal(blocked.ok, false);
  assert.match(blocked.reasons[0], /AI-Assisted/);
});
