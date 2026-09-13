import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const serviceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dockerfile = readFileSync(join(serviceRoot, 'Dockerfile'), 'utf8');
const mainSource = readFileSync(join(serviceRoot, 'src/main.rs'), 'utf8');
const configSource = readFileSync(join(serviceRoot, 'src/billing.config.rs'), 'utf8');
const webhookSource = readFileSync(join(serviceRoot, 'src/billing.webhooks.rs'), 'utf8');

test('image is reproducible and runs Billing as non-root', () => {
  assert.match(dockerfile, /^FROM rust:1\.91\.1-slim-bookworm@sha256:[a-f0-9]{64} AS builder$/m);
  assert.ok(dockerfile.includes('cargo build --locked --release --bin nvbes-billing-service'));
  assert.ok(dockerfile.includes('USER 10001:10001'));
  assert.ok(dockerfile.includes('ENTRYPOINT ["/app/billing-service"]'));
});

test('container has shallow liveness and graceful shutdown', () => {
  assert.ok(dockerfile.includes('http://127.0.0.1:8080/health/live'));
  assert.equal(dockerfile.includes('/health/ready'), false);
  assert.ok(dockerfile.includes('STOPSIGNAL SIGTERM'));
  assert.ok(mainSource.includes('action == "migrate"'));
  assert.ok(mainSource.includes('SignalKind::terminate()'));
});

test('Billing rejects live keys and live webhooks, signup is absent', () => {
  assert.equal(mainSource.includes('/auth/register'), false);
  assert.ok(configSource.includes('sk_live_'));
  assert.ok(configSource.includes('rk_live_'));
  assert.ok(webhookSource.includes('livemode'));
});

test('schema changes and synthetic smoke are explicit commands', () => {
  assert.equal(mainSource.match(/database::migrate\(&pool\)\.await\?/gu)?.length, 1);
  assert.ok(mainSource.includes('action == "synthetic-billing-smoke"'));
});
