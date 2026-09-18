import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const fullWorkflow = readFileSync('.github/workflows/validate-restore.yml', 'utf8');
const workflow = fullWorkflow.slice(
  fullWorkflow.indexOf('validate-account-restore:'),
  fullWorkflow.indexOf('validate-billing-restore:'),
);

test('Account restore is restricted to an exact approved main revision', () => {
  assert.match(workflow, /github\.ref == 'refs\/heads\/main'/u);
  assert.match(workflow, /ref: main/u);
  assert.match(workflow, /fetch-depth: 0/u);
  assert.match(workflow, /git merge-base --is-ancestor "\$APPROVED_SHA" origin\/main/u);
  assert.match(workflow, /git checkout --detach "\$APPROVED_SHA"/u);
  assert.match(workflow, /\[\[ "\$APPROVED_SHA" =~ \^\[0-9a-f\]\{40\}\$ \]\]/u);
  assert.match(workflow, /\[\[ "\$APPROVED_SHA" == "\$\(git rev-parse HEAD\)" \]\]/u);
  assert.match(workflow, /validate-account-production-restore/u);
});

test('Account restore is isolated, bounded, verified and automatically removed', () => {
  assert.match(
    workflow,
    /- name: Provision isolated restore[\s\S]*?AWS_ACCESS_KEY_ID: \$\{\{ secrets\.ACCOUNT_TERRAFORM_STATE_ACCESS_KEY \}\}[\s\S]*?terraform -chdir=infrastructure\/environments\/account-production output/u,
  );
  assert.match(workflow, /from_backup_id: \$backup_id/u);
  assert.match(workflow, /cpu_min: 0, cpu_max: 1/u);
  assert.match(workflow, /\.migrations == 1 and \.tables == 8/u);
  assert.match(workflow, /\.principal == 1/u);
  assert.match(workflow, /--file=- <<'SQL'/u);
  assert.doesNotMatch(workflow, /--command "SELECT/u);
  assert.match(workflow, /DELETE_RESTORE_DATABASE=true/u);
  assert.match(workflow, /if: \$\{\{ always\(\) \}\}/u);
});

test('Account recovery evidence contains no PII', () => {
  assert.match(workflow, /safe_integrity=.*del\(\.principal\)/u);
  assert.doesNotMatch(workflow, /SELECT \*/u);
  assert.doesNotMatch(workflow, /firstname/u);
  assert.doesNotMatch(workflow, /lastname/u);
  assert.doesNotMatch(workflow, /birthdate/u);
});
