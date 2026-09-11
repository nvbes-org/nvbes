import assert from 'node:assert/strict';

export const localRunner = ['self-hosted', 'Linux', 'ARM64', 'docker'];
export const runnerSelector =
  '${{ github.event_name == \'workflow_dispatch\' && inputs.runner == \'local\' && fromJSON(\'["self-hosted","Linux","ARM64","docker"]\') || \'ubuntu-latest\' }}';
export const candidateGuard =
  "(github.event_name != 'workflow_dispatch' || inputs.runner != 'local' || inputs.expected_sha == github.sha)";

export function fallbackJob(path) {
  assert(['.github/workflows/ci.yml', '.github/workflows/v1-testing.yml'].includes(path));
  const dependency = path.endsWith('/ci.yml') ? 'ci-gate' : 'typescript-measurement';
  return `  local-fallback:
    needs: [${dependency}]
    if: \${{ !cancelled() && needs.${dependency}.result == 'failure' && inputs.runner != 'local' && (github.event_name == 'push' || github.event_name == 'workflow_dispatch' || (github.event_name == 'pull_request' && github.event.pull_request.head.repo.full_name == github.repository && contains(fromJSON('["OWNER","MEMBER"]'), github.event.pull_request.author_association))) }}
    runs-on: [self-hosted, Linux, ARM64, docker]
    timeout-minutes: 5
    permissions:
      actions: write
      checks: read
      contents: read
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          fetch-depth: 0
          persist-credentials: false
      - name: CI/CD security gate
        run: node tools/security/check-ci-cd-security.mjs --workflow ${path}
      - name: Retry confirmed hosted startup failure once on local runners
        env:
          GITHUB_TOKEN: \${{ github.token }}
        run: node tools/ci/runner-fallback.mjs
`;
}

export function permitsFallbackDispatch(path, text) {
  if (!['.github/workflows/ci.yml', '.github/workflows/v1-testing.yml'].includes(path))
    return false;
  const block = fallbackJob(path);
  if (!text.endsWith(block)) return false;
  return !/^\s*actions:\s*write\s*$/mu.test(text.slice(0, -block.length));
}

export function fallbackPlan({
  run,
  jobs,
  annotations,
  repository,
  branchSha,
  commit,
  permission,
  existing,
}) {
  assert(
    ['.github/workflows/ci.yml', '.github/workflows/v1-testing.yml'].includes(run.path),
    'Unexpected workflow',
  );
  assert.equal(run.head_repository?.full_name, repository, 'Foreign repository');
  assert(['push', 'pull_request', 'workflow_dispatch'].includes(run.event), 'Unexpected event');
  assert.equal(run.run_attempt, 1, 'Only one automatic fallback attempt');
  assert(['admin', 'maintain', 'write'].includes(permission), 'Actor is not a repository writer');
  assert(/^[a-f0-9]{40}$/u.test(run.head_sha), 'Invalid SHA');
  assert.equal(branchSha, run.head_sha, 'Candidate branch has moved');
  assert.equal(commit.sha, run.head_sha, 'Commit response does not match candidate');
  assert.equal(commit.commit?.verification?.verified, true, 'Candidate signature is not verified');
  assert.equal(commit.commit.verification.reason, 'valid', 'Invalid candidate signature');
  assert(
    typeof run.head_branch === 'string' && /^[A-Za-z0-9][A-Za-z0-9/_.-]*$/u.test(run.head_branch),
    'Invalid branch',
  );
  const failed = jobs.filter(
    (job) => job.conclusion === 'failure' && !['ci-gate', 'local-fallback'].includes(job.name),
  );
  assert(failed.length > 0, 'No failed hosted job');
  for (const job of failed) {
    assert.equal(job.steps?.length, 0, 'Executed jobs must not trigger fallback');
    assert(!job.runner_name, 'A runner already executed the failed job');
    assert(job.labels?.includes('ubuntu-latest'), 'Not a hosted job');
    assert(
      annotations[job.id]?.some(
        ({ message }) =>
          typeof message === 'string' &&
          (message.includes(
            'The job was not started because recent account payments have failed or your spending limit needs to be increased',
          ) ||
            message.includes('The job was not started because no hosted runners are available')),
      ),
      'No confirmed hosted startup unavailability',
    );
  }
  const marker = `CI local fallback ${run.id}`;
  assert(!existing.some((entry) => entry.display_title === marker), 'Fallback already dispatched');
  return {
    ref: run.head_branch,
    inputs: { runner: 'local', expected_sha: run.head_sha, fallback_of: String(run.id) },
  };
}
