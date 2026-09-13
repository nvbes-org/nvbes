import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

export function networkSandbox() {
  if (process.platform === 'darwin') {
    return {
      command: '/usr/bin/sandbox-exec',
      args: ['-p', '(version 1)(allow default)(deny network*)'],
      cleanup() {},
    };
  }
  assert.equal(process.platform, 'linux', 'Micro-tests require a supported network sandbox');
  const directory = mkdtempSync(path.join(tmpdir(), 'nvbes-network-deny-'));
  const command = path.join(directory, 'network-deny');
  const build = spawnSync(
    'cc',
    ['-Wall', '-Wextra', '-Werror', '-O2', 'tools/rust-workspace/network-deny.c', '-o', command],
    { stdio: 'inherit' },
  );
  if (build.status !== 0) {
    rmSync(directory, { recursive: true });
    throw new Error('Cannot build the network sandbox; refusing unisolated tests');
  }
  return { command, args: [], cleanup: () => rmSync(directory, { recursive: true }) };
}

export function isolatedRun(sandbox, command, args, options = {}) {
  return spawnSync(sandbox.command, [...sandbox.args, command, ...args], {
    timeout: 120_000,
    encoding: 'utf8',
    ...options,
  });
}

export function verifySandbox(sandbox) {
  const normal = isolatedRun(sandbox, process.execPath, ['-e', 'process.exit(0)']);
  assert.equal(normal.status, 0, 'Sandbox cannot execute tests');
  for (const host of ['127.0.0.1', '::1']) {
    const probe = isolatedRun(sandbox, process.execPath, [
      '-e',
      `require('node:net').createServer().listen(0, ${JSON.stringify(host)}, () => process.exit(0));`,
    ]);
    assert(
      probe.error === undefined && (probe.signal || probe.status !== 0),
      `Network sandbox failed its ${host} negative control`,
    );
  }
}
