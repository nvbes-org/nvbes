import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const serviceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dockerfile = readFileSync(join(serviceRoot, 'Dockerfile'), 'utf8');
const serverSource = readFileSync(join(serviceRoot, 'src/api.server.rs'), 'utf8');

test('image is reproducible and runs API as non-root', () => {
  assert.match(dockerfile, /^FROM rust:1\.98\.1-slim-bookworm@sha256:[a-f0-9]{64} AS builder$/m);
  assert.ok(dockerfile.includes('cargo build --locked --release --bin nvbes-api'));
  assert.ok(dockerfile.includes('USER 10001:10001'));
  assert.ok(dockerfile.includes('ENTRYPOINT ["/app/api"]'));
});

test('container has shallow liveness and graceful shutdown', () => {
  assert.ok(dockerfile.includes('http://127.0.0.1:8080/health/live'));
  assert.equal(dockerfile.includes('/health/ready'), false);
  assert.ok(dockerfile.includes('STOPSIGNAL SIGTERM'));
  assert.ok(serverSource.includes('shutdown_signal()'));
  assert.ok(serverSource.includes('SignalKind::terminate()'));
});
