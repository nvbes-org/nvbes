#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const errors = [];

function requirePath(path) {
  if (!existsSync(path)) errors.push(`${path}: missing`);
}

function requireText(path, text) {
  if (!existsSync(path)) {
    errors.push(`${path}: missing`);
    return;
  }
  if (!readFileSync(path, 'utf8').includes(text)) errors.push(`${path}: missing ${text}`);
}

requireText(
  'deploy/security/kyverno/verify-nvbes-images.yaml',
  'require-keyless-release-signature',
);
requireText(
  'deploy/security/kyverno/require-restricted-containers.yaml',
  'readOnlyRootFilesystem: true',
);
requireText('.github/workflows-archive/container-release.yml', 'cosign sign --yes');
requireText('.github/workflows-archive/container-release.yml', 'cosign attest --yes');
requireText('.github/workflows-archive/container-release.yml', 'format: cyclonedx');
requireText(
  '.github/workflows-archive/container-release.yml',
  'include: $' + '{{ fromJSON(needs.detect-affected.outputs.rust) }}',
);
requireText(
  '.github/workflows-archive/container-release.yml',
  'node tools/ci/affected-applications.mjs',
);
requireText('.dockerignore', '.env');
requireText('.dockerignore', 'target');

const catalogPath = 'tools/ci/deployable-applications.json';
requirePath(catalogPath);
const catalog = existsSync(catalogPath)
  ? JSON.parse(readFileSync(catalogPath, 'utf8'))
  : { rust: [] };
const dockerfiles = [
  ...new Set([
    ...catalog.rust.map(({ dockerfile }) => dockerfile),
    'apps/platform-operations-service/Dockerfile',
  ]),
];

for (const dockerfile of dockerfiles) {
  requireText(dockerfile, 'FROM rust:1.91.1-slim-bookworm@sha256:');
  requireText(dockerfile, 'FROM debian:bookworm-slim@sha256:');
  requireText(dockerfile, 'cargo build --locked --release');
  requireText(dockerfile, 'USER 10001:10001');
  requireText(dockerfile, 'STOPSIGNAL SIGTERM');
  requireText(dockerfile, '/health/live');
  if (existsSync(dockerfile)) {
    const content = readFileSync(dockerfile, 'utf8');
    if (content.includes('/health/ready')) {
      errors.push(
        `${dockerfile}: HEALTHCHECK must not probe /health/ready (use shallow /health/live)`,
      );
    }
    for (const secretPattern of ['sk_live_', 'whsec_', 'rk_live_']) {
      if (content.includes(secretPattern)) {
        errors.push(`${dockerfile}: contains hardcoded secret pattern ${secretPattern}`);
      }
    }
  }
}

const releaseImages = [
  'nvbes-account-service',
  'nvbes-billing-service',
  'nvbes-email-worker',
  'nvbes-identity-service',
  'nvbes-trust-risk-service',
];

const catalogImages = new Set(catalog.rust.map(({ image }) => image));
for (const image of releaseImages) {
  if (!catalogImages.has(image)) {
    errors.push(`${catalogPath}: missing image ${image}`);
  }
}
if (catalogImages.size !== releaseImages.length) {
  errors.push(
    `${catalogPath}: expected ${releaseImages.length} unique Rust images, found ${catalogImages.size}`,
  );
}

if (errors.length > 0) {
  console.error('Container/deploy manifest checks failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('Container/deploy manifests: ok');
