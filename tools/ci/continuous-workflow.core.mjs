import assert from 'node:assert/strict';
import { parse } from 'yaml';
import { lanes } from './scope-plan.mjs';

export function validateContinuousWorkflow(text, setupText) {
  const workflow = parse(text);
  const jobs = workflow.jobs;
  const shell = 'bash --noprofile --norc -euo pipefail {0}';
  assert.deepEqual(
    Object.keys(jobs).sort(),
    ['authorize-cache', 'scope', ...lanes, 'typescript-measurement', 'ci-gate'].sort(),
  );
  assert.deepEqual(workflow.defaults, { run: { shell } });
  assert.deepEqual(workflow.permissions, {
    actions: 'read',
    contents: 'read',
    'pull-requests': 'read',
  });
  assert.equal(jobs['ci-gate'].if, 'always()');
  assert.deepEqual(jobs['ci-gate'].needs, ['authorize-cache', 'scope', ...lanes]);
  assert.equal(jobs['ci-gate'].steps.at(-1).run, 'node tools/ci/ci-gate.mjs');
  for (const [name, job] of Object.entries(jobs)) {
    assert.equal(job['runs-on'], 'ubuntu-latest');
    assert.ok(job['timeout-minutes'] > 0 && job['timeout-minutes'] <= 45);
    assert.equal(job['continue-on-error'], undefined);
    assert.equal(job.defaults, undefined);
    for (const key of ['BASH_ENV', 'NODE_OPTIONS', 'PATH'])
      assert.equal(job.env?.[key] ?? workflow.env?.[key], undefined);
    for (const step of job.steps) {
      assert.equal(step['continue-on-error'], undefined);
      assert.equal(step.shell, undefined);
      if (step.uses?.startsWith('actions/checkout@') && name !== 'authorize-cache')
        assert.deepEqual(step.with, { 'fetch-depth': 0, 'persist-credentials': false });
    }
  }
  const finops = jobs.scope.steps.filter((step) => step.run === 'pnpm check:finops');
  assert.equal(finops.length, 1, 'exact FinOps gate required in scope');
  assert.deepEqual(finops[0], {
    name: 'Enforce FinOps contract',
    if: "steps.preflight.outputs.graph-required == 'true'",
    run: 'pnpm check:finops',
  });
  for (const lane of lanes) {
    const job = jobs[lane];
    assert.deepEqual(job.needs, ['authorize-cache', 'scope']);
    const expression = `!cancelled() && needs.scope.result == 'success' && needs.authorize-cache.result == 'success' && needs.scope.outputs.${lane}-required == 'true'`;
    assert.equal(job.if, `\${{ ${expression} }}`);
    const setup = job.steps.find((step) => step.uses === './.github/actions/ci-setup');
    assert.ok(setup, 'lane setup required');
    assert.equal(setup.with.rust, String(['rust', 'database'].includes(lane)));
    assert.equal(setup.with.terraform, String(lane === 'terraform'));
    assert.equal(setup.with.node, String(lane !== 'rust'));
    assert.equal(setup.if, undefined);
    const security = job.steps.findIndex(
      (step) =>
        step.run ===
        'node tools/security/check-ci-cd-security.mjs --workflow .github/workflows/ci.yml',
    );
    assert.ok(security >= 0 && security < job.steps.indexOf(setup));
    assert.equal(job.steps[security].if, undefined);
    assert.ok(job.environment.name.includes("needs.authorize-cache.outputs.trusted == 'true'"));
    assert.ok(job.environment.name.includes("'ci-no-secrets'"));
  }
  const measurement = jobs['typescript-measurement'];
  assert.equal(measurement.if, "github.event_name == 'workflow_dispatch'");
  assert.deepEqual(measurement.environment, { name: 'ci-no-secrets', deployment: false });
  assert.deepEqual(measurement.env, {
    NVBES_COVERAGE_PACKAGE: 'http-client',
    NVBES_MUTATION_PACKAGE: 'http-client',
  });
  const measurementSetup = measurement.steps.find(
    (step) => step.uses === './.github/actions/ci-setup',
  );
  assert.deepEqual(measurementSetup.with, {
    node: 'true',
    rust: 'false',
    terraform: 'false',
    caches: 'pnpm',
  });
  const measurementSecurity = measurement.steps.findIndex(
    (step) =>
      step.run ===
      'node tools/security/check-ci-cd-security.mjs --workflow .github/workflows/ci.yml',
  );
  assert.ok(
    measurementSecurity >= 0 && measurementSecurity < measurement.steps.indexOf(measurementSetup),
  );
  const published = measurement.steps.find(
    (step) => step.name === 'Publish HTTP client measurement',
  );
  assert.match(published.uses, /^actions\/upload-artifact@[a-f0-9]{40}$/u);
  assert.equal(published.with['retention-days'], 7);
  assert.equal(published.with['if-no-files-found'], 'error');
  const preflight = jobs.scope.steps.findIndex((step) => step.id === 'preflight');
  const setup = jobs.scope.steps.findIndex((step) => step.uses === './.github/actions/ci-setup');
  const finopsIndex = jobs.scope.steps.findIndex((step) => step.run === 'pnpm check:finops');
  assert.ok(preflight >= 0 && preflight < setup);
  assert.ok(setup < finopsIndex);
  assert.equal(jobs.scope.steps[setup].if, "steps.preflight.outputs.graph-required == 'true'");
  const composite = parse(setupText);
  for (const step of composite.runs.steps) {
    assert.equal(step['continue-on-error'], undefined);
    if (step.run) assert.equal(step.shell, shell);
    if (step.uses) assert.match(step.uses, /@[a-f0-9]{40}$/u);
    if (step.run?.includes('pnpm install'))
      assert.equal(step.run.trim(), 'pnpm install --frozen-lockfile --prefer-offline');
  }
}
