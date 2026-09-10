import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

test('concurrent local starters share one complete private resource key without logging it', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'nvbes-resource-test-'));
  const script = fileURLToPath(new URL('./dev-identity-resources.mjs', import.meta.url));
  try {
    const run = () =>
      new Promise((resolve, reject) => {
        const child = spawn(process.execPath, [script, directory]);
        let stdout = '';
        let stderr = '';
        child.stdout.on('data', (chunk) => {
          stdout += chunk;
        });
        child.stderr.on('data', (chunk) => {
          stderr += chunk;
        });
        child.on('error', reject);
        child.on('close', (code) => {
          try {
            assert.equal(code, 0);
            assert.equal(stdout, '');
            assert.equal(stderr, '');
            resolve();
          } catch (error) {
            reject(error);
          }
        });
      });
    await Promise.all(Array.from({ length: 8 }, run));
    const path = join(directory, 'identity-billing-resource.secret');
    const before = readFileSync(path, 'utf8');
    assert.equal(Buffer.from(before, 'base64url').length, 32);
    assert.equal(statSync(path).mode & 0o777, 0o600);
    await run();
    assert.equal(readFileSync(path, 'utf8'), before);
    const account = join(directory, 'identity-account-resource.secret');
    assert.equal(statSync(account).mode & 0o777, 0o600);
    assert.equal(Buffer.from(readFileSync(account, 'utf8'), 'base64url').length, 32);
    assert.notEqual(readFileSync(account, 'utf8'), before);
    const authority = join(directory, 'identity-billing-authorization-resource.secret');
    const authorityBefore = readFileSync(authority, 'utf8');
    assert.match(authorityBefore, /^[0-9a-f]{64}$/);
    assert.equal(statSync(authority).mode & 0o777, 0o600);
    await run();
    assert.equal(readFileSync(authority, 'utf8'), authorityBefore);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('the local key loader exports matching Billing and Identity credentials', () => {
  const directory = mkdtempSync(join(tmpdir(), 'nvbes-resource-loader-'));
  const helper = fileURLToPath(new URL('./dev-identity-keys.sh', import.meta.url));
  const check = `const assert = require('node:assert/strict');
    const entry = JSON.parse(process.env.NVBES_IDENTITY_RESOURCE_SERVERS_JSON)[0];
    assert.equal(entry.secret, process.env.NVBES_BILLING_IDENTITY_RESOURCE_SECRET);
    assert.equal(entry.client_id, process.env.NVBES_BILLING_IDENTITY_RESOURCE_CLIENT_ID);
    assert.equal(entry.audience, 'nvbes-billing-service');
    const account = JSON.parse(process.env.NVBES_IDENTITY_RESOURCE_SERVERS_JSON)[1];
    assert.equal(account.secret, process.env.NVBES_ACCOUNT_IDENTITY_RESOURCE_SECRET);
    assert.equal(account.client_id, process.env.NVBES_ACCOUNT_IDENTITY_RESOURCE_CLIENT_ID);
    assert.equal(account.audience, 'nvbes-account-service');
    assert.notEqual(account.secret, entry.secret);
    assert.equal(process.env.NVBES_IDENTITY_TOKEN_AUDIENCES, 'nvbes-account-service,nvbes-billing-service');
    assert.match(process.env.NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET, /^[0-9a-f]{64}$/);
    assert.equal(process.env.NVBES_BILLING_ACCOUNT_ORIGIN, 'http://127.0.0.1:3070');`;
  try {
    const result = spawnSync(
      'bash',
      [
        '-c',
        'set -euo pipefail; source "$1"; load_dev_identity_keys; "$2" -e "$3"',
        'resource-test',
        helper,
        process.execPath,
        check,
      ],
      {
        encoding: 'utf8',
        env: {
          PATH: process.env.PATH,
          ROOT_DIR: directory,
          NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM: 'unused-fixture',
          NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM: 'unused-fixture',
        },
      },
    );
    assert.equal(result.status, 0);
    assert.equal(result.stdout, '');
    assert.equal(result.stderr, '');
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
