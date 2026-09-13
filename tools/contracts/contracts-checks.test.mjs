import assert from 'node:assert/strict';
import test from 'node:test';
import {
  checkEventDocuments,
  checkProtoBreaking,
  checkProtoDocuments,
  runContractChecks,
} from './checks.mjs';

test('runContractChecks passes on canonical codebase', () => {
  const errors = runContractChecks();
  assert.deepEqual(errors, []);
});

test('checkProtoDocuments returns empty errors on valid proto files', () => {
  const errors = [];
  checkProtoDocuments(errors);
  assert.deepEqual(errors, []);
});

test('checkEventDocuments validates all schema invariants', () => {
  const errors = [];
  checkEventDocuments(errors);
  assert.deepEqual(errors, []);
});

test('checkProtoBreaking skips when CI=true and no explicit target is set', () => {
  const errors = [];
  checkProtoBreaking(errors, { CI: 'true' });
  assert.deepEqual(errors, []);
});

test('checkProtoBreaking against origin/main passes on valid changes', () => {
  const errors = [];
  checkProtoBreaking(errors, {
    NVBES_BUF_BREAKING_AGAINST: '.git#branch=origin/main,subdir=contracts/protobuf',
  });
  assert.deepEqual(errors, []);
});
