import assert from 'node:assert/strict';
import test from 'node:test';
import { selectAffectedApplications } from './affected-applications.core.mjs';

const workspaceRoot = '/workspace';
const catalog = {
  rust: [
    {
      project: 'identity-service',
      package: 'nvbes-identity-service',
      dockerfile: 'infrastructure/docker/rust-application.Dockerfile',
    },
    {
      project: 'email-worker',
      package: 'nvbes-email-worker',
      dockerfile: 'apps/email-worker/Dockerfile',
    },
  ],
  web: [{ project: 'identity-web' }, { project: 'account-web' }],
};
const cargoMetadata = {
  workspace_members: ['identity', 'email', 'email-lib'],
  packages: [
    {
      id: 'identity',
      name: 'nvbes-identity-service',
      manifest_path: '/workspace/apps/identity-service/Cargo.toml',
    },
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
      { id: 'identity', deps: [{ pkg: 'email-lib' }] },
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
    ['identity-service', 'email-worker'],
  );
});

void test('Nx selects one affected web application', () => {
  const result = select({
    changedPaths: ['apps/identity-web/src/main.tsx'],
    nxAffected: ['identity-web'],
  });
  assert.deepEqual(
    result.web.map(({ project }) => project),
    ['identity-web'],
  );
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

void test('the generic Dockerfile selects only applications using it', () => {
  const result = select({
    changedPaths: ['infrastructure/docker/rust-application.Dockerfile'],
    mode: 'containers',
  });
  assert.deepEqual(
    result.rust.map(({ project }) => project),
    ['identity-service'],
  );
});

void test('unrelated documentation selects nothing', () => {
  const result = select({ changedPaths: ['docs/README.md'] });
  assert.deepEqual(result, { rust: [], web: [] });
});
