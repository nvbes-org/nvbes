import assert from 'node:assert/strict';
import test from 'node:test';
import { isActiveRuntimePath, isArchivedPath } from './generate-deterministic-docs.mjs';

test('classifies only archive-rooted paths as archived', () => {
  assert.equal(isArchivedPath('archive'), true);
  assert.equal(isArchivedPath('archive/apps/cloud-service'), true);
  assert.equal(isArchivedPath('apps/email-worker'), false);
  assert.equal(isArchivedPath('libs/rust/cloud'), false);
  assert.equal(isArchivedPath('docs/archive-notes.md'), false);
});

test('treats only OpenAPI files under active runtime apps as active', () => {
  assert.equal(isActiveRuntimePath('apps/identity-service/openapi.json'), true);
  assert.equal(isActiveRuntimePath('archive/apps/identity-service/openapi.json'), false);
  assert.equal(isActiveRuntimePath('archive/docs/api/openapi/cloud-public-v1.openapi.json'), false);
  assert.equal(isActiveRuntimePath('libs/ts/identity-sdk-core/openapi.json'), false);
  assert.equal(isActiveRuntimePath('apps/docs/openapi.json'), false);
});
