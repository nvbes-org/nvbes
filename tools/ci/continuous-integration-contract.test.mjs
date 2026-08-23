import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');

test('continuous CI runs on every active delivery branch', () => {
  assert.match(workflow, /branches: \['main', 'staging', 'dev'\]/u);
  assert.match(workflow, /workflow_dispatch:/u);
  assert.match(workflow, /email-quality:/u);
});

test('continuous CI validates only the active email runtime', () => {
  assert.match(workflow, /POSTGRES_DB: nvbes_email_test/u);
  assert.match(workflow, /job\.services\.postgres\.ports\[5432\]/u);
  assert.match(workflow, /host\.docker\.internal/u);
  assert.match(workflow, /pnpm test:pre-deploy/u);
  assert.match(workflow, /pnpm test/u);
  assert.match(workflow, /bash scripts\/test-email-worker-database\.sh/u);
  assert.match(workflow, /terraform -chdir=infrastructure\/stacks\/email\/production test/u);
  assert.match(workflow, /node --test apps\/email-worker\/tests\/container-contract\.test\.mjs/u);
  assert.doesNotMatch(workflow, /identity-migration-checks/u);
  assert.doesNotMatch(workflow, /test-identity-service-migrations/u);
  assert.doesNotMatch(workflow, /pnpm test:unit/u);
});

test('continuous CI avoids remote dependency caches on self-hosted runners', () => {
  assert.doesNotMatch(workflow, /Swatinem\/rust-cache@/u);
  assert.doesNotMatch(workflow, /^\s+cache: pnpm\s*$/mu);
  assert.match(workflow, /node --test tools\/ci\/continuous-integration-contract\.test\.mjs/u);
});
