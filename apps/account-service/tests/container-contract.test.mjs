import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const serviceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dockerfile = readFileSync(join(serviceRoot, 'Dockerfile'), 'utf8');
const mainSource = readFileSync(join(serviceRoot, 'src/main.rs'), 'utf8');
const authSource = readFileSync(join(serviceRoot, 'src/account.auth.rs'), 'utf8');
const tokenSource = readFileSync(join(serviceRoot, 'src/account.auth.tokens.rs'), 'utf8');
const privacySource = readFileSync(join(serviceRoot, 'src/account.privacy.rs'), 'utf8');

test('image is reproducible and runs Account as non-root', () => {
  assert.match(dockerfile, /^FROM rust:1\.91\.1-slim-bookworm@sha256:[a-f0-9]{64} AS builder$/m);
  assert.ok(dockerfile.includes('cargo build --locked --release --bin nvbes-account-service'));
  assert.ok(dockerfile.includes('USER 10001:10001'));
  assert.ok(dockerfile.includes('ENTRYPOINT ["/app/account-service"]'));
});

test('container has shallow liveness and graceful shutdown', () => {
  assert.ok(dockerfile.includes('http://127.0.0.1:8080/health/live'));
  assert.ok(dockerfile.includes('STOPSIGNAL SIGTERM'));
  assert.ok(mainSource.includes('action == "migrate"'));
  assert.ok(mainSource.includes('SignalKind::terminate()'));
});

test('Account remains closed and destructive privacy actions require MFA step-up', () => {
  assert.equal(mainSource.includes('/auth/register'), false);
  assert.equal(mainSource.includes('/checkout'), false);
  assert.ok(tokenSource.includes('matches!(m.as_str(), "totp" | "webauthn")'));
  assert.ok(authSource.includes('SELECT clock_timestamp() < $1'));
  assert.ok(privacySource.match(/principal\.require_step_up\(\)\?/gu)?.length >= 4);
  assert.equal(privacySource.match(/\.require_step_up_at_database\(/gu)?.length, 6);
});

test('schema changes and privacy work are explicit commands', () => {
  assert.equal(mainSource.match(/database::migrate\(&pool\)\.await\?/gu)?.length, 1);
  assert.ok(mainSource.includes('action == "process-privacy-jobs"'));
  assert.ok(mainSource.includes('action == "synthetic-account-smoke"'));
});
