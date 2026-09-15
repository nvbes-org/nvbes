import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const fullWorkflow = readFileSync('.github/workflows/validate-restore.yml', 'utf8');
const workflow = fullWorkflow.slice(
  fullWorkflow.indexOf('validate-email-restore:'),
  fullWorkflow.indexOf('validate-identity-restore:'),
);

test('Email restore is restricted to an exact approved main revision', () => {
  assert.match(workflow, /github\.ref == 'refs\/heads\/main'/u);
  assert.match(workflow, /inputs\.service == 'email'/u);
  assert.match(workflow, /ref: main/u);
  assert.match(workflow, /fetch-depth: 0/u);
  assert.match(workflow, /git merge-base --is-ancestor "\$APPROVED_SHA" origin\/main/u);
  assert.match(workflow, /git checkout --detach "\$APPROVED_SHA"/u);
  assert.match(workflow, /\[\[ "\$APPROVED_SHA" =~ \^\[0-9a-f\]\{40\}\$ \]\]/u);
  assert.match(workflow, /\[\[ "\$APPROVED_SHA" == "\$\(git rev-parse HEAD\)" \]\]/u);
  assert.match(workflow, /validate-email-production-restore/u);
});

test('Email restore is isolated, bounded, verified and automatically removed', () => {
  assert.match(
    workflow,
    /- name: Provision a fresh isolated restore[\s\S]*?AWS_ACCESS_KEY_ID: \$\{\{ secrets\.EMAIL_TERRAFORM_STATE_ACCESS_KEY \}\}/u,
  );
  assert.match(workflow, /from_backup_id: \$from_backup_id/u);
  assert.match(workflow, /cpu_min: 0, cpu_max: 1/u);
  assert.match(workflow, /\.migrations == 4 and \.tables == 5/u);
  assert.match(workflow, /\.constraints_valid == true/u);
  assert.match(workflow, /DELETE_RESTORE_DATABASE=true/u);
  assert.match(workflow, /if: \$\{\{ always\(\) \}\}/u);
});

test('Email recovery protects message confidentiality and records RTO', () => {
  assert.match(workflow, /safe_integrity=.*del\(\.messages, \.attempts, \.provider_events\)/u);
  assert.match(
    workflow,
    /restore_rto_seconds=\$\(\(restore_completed_at - RESTORE_STARTED_AT\)\)/u,
  );
  assert.match(workflow, /--request DELETE[\s\S]*?https:\/\/api\.scaleway\.com\/serverless-sqldb/u);
});
