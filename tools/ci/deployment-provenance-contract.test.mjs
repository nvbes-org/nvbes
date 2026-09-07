import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const deployableWorkflows = ['account', 'billing', 'email', 'identity', 'trust-risk'];

test('Terraform validation serializes shared provider installation on cold runners', () => {
  const lane = readFileSync('tools/ci/run-lane.mjs', 'utf8');
  assert.match(lane, /mkdirSync\(process\.env\.TF_PLUGIN_CACHE_DIR, \{ recursive: true \}\)/u);
  assert.ok(
    lane.includes("selected('terraform:validate', plan.terraform, plan.baseline.terraform, 1)"),
  );
});

test('state policy fits Scaleway limits with shared listing and isolated object access', () => {
  const module = readFileSync('infrastructure/modules/terraform-state-backend/main.tf', 'utf8');
  assert.match(module, /Action\s*= \["s3:ListBucket"\]/u);
  assert.match(module, /Resource\s*= \[scaleway_object_bucket\.terraform_state\.name\]/u);
  assert.match(module, /Condition = merge\(local\.tls_condition,/u);
  assert.ok(
    module.includes(
      '"s3:prefix" = [for stack in local.all_state_stacks : "${var.environment}/${stack}/*"]',
    ),
  );
  assert.match(module, /Sid\s*= "ListDeclaredStatePrefixes"/u);
  assert.match(module, /length\(local\.all_state_stacks\) \+ 1 <= 10/u);
  assert.ok(
    module.includes('Principal = { SCW = "application_id:${local.state_application_ids[stack]}" }'),
  );
  assert.match(module, /"aws:SecureTransport" = "true"/u);
});

for (const service of deployableWorkflows) {
  const path = `.github/workflows/deploy-${service}.yml`;
  const workflow = readFileSync(path, 'utf8');
  test(`${service} deployment promotes only an exactly validated main revision`, () => {
    assert.match(workflow, /^  ci-provenance:\n/mu);
    assert.match(workflow, /node tools\/ci\/verify-ci-provenance\.mjs/u);
    assert.match(workflow, /GITHUB_TOKEN: \$\{\{ github\.token \}\}/u);
    assert.match(workflow, /needs: \[ci-provenance\]/u);
    assert.doesNotMatch(workflow, /^  ci-test-gate:/mu);
  });

  test(`${service} deployment does not rebuild its CI gate`, () => {
    const gate = workflow.slice(
      workflow.indexOf('  ci-provenance:'),
      workflow.indexOf('  build-scan-sign:'),
    );
    assert.doesNotMatch(gate, /pnpm install|cargo (?:check|test)|terraform .*validate/u);
  });
}

for (const root of [
  'infrastructure/bootstrap/production',
  'infrastructure/bootstrap/production/initial',
]) {
  test(`${root} grants state access to every deployable service`, () => {
    const bootstrap = readFileSync(`${root}/main.tf`, 'utf8');
    const managed = bootstrap.match(/state_stacks = \[([^\]]+)\]/u)?.[1] ?? '';
    const external = bootstrap.match(/external_state_application_ids = \{([^}]+)\}/u)?.[1] ?? '';
    for (const service of deployableWorkflows) {
      assert.ok(
        managed.includes(`"${service}"`) ||
          new RegExp(`\\b${service}\\s*=\\s*"[0-9a-f-]{36}"`, 'u').test(external),
        `${service} is missing from the state bucket policy`,
      );
    }
    assert.match(external, /account\s*=\s*"fdcde0a1-e8f3-4a62-8a36-843b18dd515c"/u);
    assert.ok(!managed.includes('"account"'), 'reuse the existing Account IAM identity');
  });
}

test('state access reconciliation is manual and targets only the state policy', () => {
  const workflow = readFileSync('.github/workflows/rotate-ci-cache-credentials.yml', 'utf8');
  assert.match(workflow, /reconcile_state_access:[\s\S]*?default: false/u);
  for (const name of ['Plan declared state access', 'Apply declared state access']) {
    assert.ok(
      workflow.includes(`name: ${name}\n        if: inputs.reconcile_state_access == true`),
    );
  }
  assert.match(
    workflow,
    /-target=module\.terraform_state\.scaleway_object_bucket_policy\.terraform_state\s+-out=state-access\.tfplan/u,
  );
  assert.match(workflow, /apply\s+-lock-timeout=5m\s+state-access\.tfplan/u);
  for (const name of [
    'Plan isolated cache provisioning and rotation',
    'Apply reviewed cache rotation plan',
  ]) {
    assert.ok(
      workflow.includes(`name: ${name}\n        if: inputs.reconcile_state_access != true`),
    );
  }
});
