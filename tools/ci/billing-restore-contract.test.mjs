import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const fullWorkflow = readFileSync('.github/workflows/validate-restore.yml', 'utf8');
const workflow = fullWorkflow.slice(
  fullWorkflow.indexOf('validate-billing-restore:'),
  fullWorkflow.indexOf('validate-email-restore:'),
);

test('Billing restore is restricted to an exact approved main revision', () => {
  assert.match(workflow, /github\.ref == 'refs\/heads\/main'/u);
  assert.match(workflow, /inputs\.service == 'billing'/u);
  assert.match(workflow, /ref: main/u);
  assert.match(workflow, /fetch-depth: 0/u);
  assert.match(workflow, /git merge-base --is-ancestor "\$APPROVED_SHA" origin\/main/u);
  assert.match(workflow, /git checkout --detach "\$APPROVED_SHA"/u);
  assert.match(workflow, /\[\[ "\$APPROVED_SHA" =~ \^\[0-9a-f\]\{40\}\$ \]\]/u);
  assert.match(workflow, /\[\[ "\$APPROVED_SHA" == "\$\(git rev-parse HEAD\)" \]\]/u);
  assert.match(workflow, /validate-billing-production-restore/u);
});

test('Billing restore is isolated, bounded, verified and automatically removed', () => {
  assert.match(
    workflow,
    /- name: Provision isolated restore[\s\S]*?AWS_ACCESS_KEY_ID: \$\{\{ secrets\.BILLING_TERRAFORM_STATE_ACCESS_KEY \}\}[\s\S]*?terraform -chdir=infrastructure\/environments\/billing-production output/u,
  );
  assert.match(workflow, /from_backup_id: \$backup_id/u);
  assert.match(workflow, /cpu_min: 0, cpu_max: 1/u);
  assert.match(workflow, /\.migrations >= 1 and \.tables == 8/u);
  assert.match(workflow, /\.constraints_valid == true/u);
  assert.match(workflow, /--file=- <<'SQL'/u);
  assert.match(workflow, /DELETE_RESTORE_DATABASE=true/u);
  assert.match(workflow, /if: \$\{\{ always\(\) \}\}/u);
});

test('Billing recovery records RTO and deletes provisioned database', () => {
  assert.match(workflow, /rto_seconds=\$\(\( \$\(date -u \+%s\) - RESTORE_STARTED_AT \)\)/u);
  assert.match(workflow, /--request DELETE[\s\S]*?https:\/\/api\.scaleway\.com\/serverless-sqldb/u);
});
