import assert from 'node:assert/strict';
import test from 'node:test';
import { evaluatePullRequestCacheTrust, isVerifiedGpgCommit } from './authorize-pr-cache.core.mjs';

function fixture() {
  const event = {
    repository: { full_name: 'nvbes-org/nvbes' },
    sender: { login: 'internal' },
    pull_request: {
      number: 42,
      author_association: 'MEMBER',
      user: { login: 'internal' },
      head: {
        sha: 'b'.repeat(40),
        repo: { full_name: 'nvbes-org/nvbes' },
      },
    },
  };
  const commits = [
    {
      sha: 'b'.repeat(40),
      commit: {
        verification: {
          verified: true,
          reason: 'valid',
          signature: '-----BEGIN PGP SIGNATURE-----\nvalid',
        },
      },
    },
  ];
  return { event, commits, files: [{ filename: 'libs/rust/core/src/lib.rs' }] };
}

test('recognizes only valid GitHub-verified GPG commits', () => {
  const { commits } = fixture();
  assert.equal(isVerifiedGpgCommit(commits[0]), true);
  assert.equal(
    isVerifiedGpgCommit({
      ...commits[0],
      commit: {
        verification: { verified: true, reason: 'valid', signature: 'SSH' },
      },
    }),
    false,
  );
});

test('authorizes an internal same-repository pull request with signed commits', () => {
  const { event, commits, files } = fixture();
  assert.deepEqual(evaluatePullRequestCacheTrust(event, commits, files), {
    trusted: true,
    reasons: [],
  });
});

test('fails closed for forks, outsiders, unsigned heads, and trust-boundary changes', () => {
  const { event, commits } = fixture();
  event.pull_request.head.repo.full_name = 'outside/fork';
  event.pull_request.author_association = 'CONTRIBUTOR';
  commits[0].commit.verification.verified = false;
  const result = evaluatePullRequestCacheTrust(event, commits, [
    { filename: '.github/workflows/ci.yml' },
  ]);
  assert.equal(result.trusted, false);
  assert.equal(result.reasons.length, 4);
});

test('rejects a stale commit list and a different workflow actor', () => {
  const { event, commits, files } = fixture();
  event.sender.login = 'another-member';
  commits[0].sha = 'a'.repeat(40);
  const result = evaluatePullRequestCacheTrust(event, commits, files);
  assert.equal(result.trusted, false);
  assert.equal(result.reasons.length, 2);
});

test('Dependabot remains in the no-secrets context', () => {
  const { event, commits, files } = fixture();
  event.sender.login = 'dependabot[bot]';
  event.pull_request.user.login = 'dependabot[bot]';
  event.pull_request.author_association = 'NONE';
  assert.equal(evaluatePullRequestCacheTrust(event, commits, files).trusted, false);
});

test('composite actions and compilation-cache setup are trust boundaries', () => {
  const { event, commits } = fixture();
  for (const filename of [
    '.github/actions/ci-setup/action.yml',
    'tools/ci/configure-sccache.mjs',
    'tools/ci/scaleway-cache-manager.mjs',
  ]) {
    assert.equal(evaluatePullRequestCacheTrust(event, commits, [{ filename }]).trusted, false);
  }
});
