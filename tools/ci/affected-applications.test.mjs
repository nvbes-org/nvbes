import assert from 'node:assert/strict';
import test from 'node:test';
import { selectAffectedApplications } from './affected-applications.core.mjs';

const workspaceRoot = '/workspace';
const catalog = {
  rust: [
    {
      project: 'email-worker',
      package: 'nvbes-email-worker',
      dockerfile: 'apps/email-worker/Dockerfile',
    },
  ],
  web: [],
};
const cargoMetadata = {
  workspace_members: ['email', 'email-lib'],
  packages: [
    {
      id: 'email',
      name: 'nvbes-email-worker',
      manifest_path: '/workspace/apps/email-worker/Cargo.toml',
    },
    {
      id: 'email-lib',
      name: 'nvbes-email',
      manifest_path: '/workspace/libs/rust/email/Cargo.toml',
    },
  ],
  resolve: {
    nodes: [
      { id: 'email', deps: [{ pkg: 'email-lib' }] },
      { id: 'email-lib', deps: [] },
    ],
  },
};

function select({ changedPaths, nxAffected = [], mode = 'ci' }) {
  return selectAffectedApplications({
    catalog,
    changedPaths,
    nxAffected,
    cargoMetadata,
    workspaceRoot,
    mode,
  });
}

void test('a Rust application change selects only that application', () => {
  const result = select({
    changedPaths: ['apps/email-worker/src/main.rs'],
    nxAffected: ['email-worker'],
  });
  assert.deepEqual(
    result.rust.map(({ project }) => project),
    ['email-worker'],
  );
});

void test('a shared Rust crate selects only its transitive consumers', () => {
  const result = select({ changedPaths: ['libs/rust/email/src/lib.rs'] });
  assert.deepEqual(
    result.rust.map(({ project }) => project),
    ['email-worker'],
  );
});

void test('web mode stays empty while no web application is active', () => {
  const result = select({
    changedPaths: ['libs/ts/email-ui/src/index.ts'],
    nxAffected: ['email-ui'],
    mode: 'web',
  });
  assert.deepEqual(result, { rust: [], web: [] });
});

void test('one custom Dockerfile selects only its image', () => {
  const result = select({
    changedPaths: ['apps/email-worker/Dockerfile'],
    mode: 'containers',
  });
  assert.deepEqual(
    result.rust.map(({ project }) => project),
    ['email-worker'],
  );
});

void test('a global Rust manifest selects the active application', () => {
  const result = select({
    changedPaths: ['Cargo.toml'],
  });
  assert.deepEqual(
    result.rust.map(({ project }) => project),
    ['email-worker'],
  );
});

void test('unrelated documentation selects nothing', () => {
  const result = select({ changedPaths: ['docs/README.md'] });
  assert.deepEqual(result, { rust: [], web: [] });
});
