import assert from 'node:assert/strict';

export const runnerSelector = `\${{ github.event_name == 'workflow_dispatch' && inputs.runner == 'local' && fromJSON('["self-hosted", "Linux", "ARM64", "docker"]') || 'ubuntu-latest' }}`;
export const localGuardCondition =
  "github.event_name == 'workflow_dispatch' && inputs.runner == 'local'";
export const localGuardCommand = `[[ "$RUNNER_OS" == "Linux" && "$(uname -s)" == "Linux" ]] || exit 1
[[ "$NVBES_CI_EXPECTED_SHA" =~ ^[0-9a-f]{40}$ && "$GITHUB_SHA" == "$NVBES_CI_EXPECTED_SHA" ]] || exit 1
[[ "$(docker info --format '{{.OSType}}')" == "linux" ]] || exit 1`;

export function validateRunnerFallback(workflow) {
  const inputs = workflow.on.workflow_dispatch.inputs;
  assert.equal(inputs.runner.type, 'choice');
  assert.deepEqual(inputs.runner.options, ['github', 'local']);
  assert.equal(inputs.runner.default, 'github');
  assert.equal(inputs.runner.required, true);
  assert.equal(inputs.candidate_sha.type, 'string');
  assert.equal(workflow.env.NVBES_CI_EXPECTED_SHA, '${{ inputs.candidate_sha }}');
  for (const job of Object.values(workflow.jobs)) {
    assert.equal(job['runs-on'], runnerSelector);
    const guard = job.steps[0];
    assert.equal(guard.name, 'Verify local runner and candidate');
    assert.equal(guard.if, localGuardCondition);
    assert.equal(guard.run.trim(), localGuardCommand);
    assert.equal(guard['continue-on-error'], undefined);
    assert.equal(guard.env, undefined);
    assert.equal(job.env?.NVBES_CI_EXPECTED_SHA, undefined);
    assert.equal(job.env?.RUNNER_OS, undefined);
    assert.equal(job.env?.GITHUB_SHA, undefined);
  }
}
