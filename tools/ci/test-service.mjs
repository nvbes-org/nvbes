import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { hostname } from 'node:os';

export function serviceName(kind, env = process.env) {
  assert(['postgres', 'redis'].includes(kind), 'Unknown test service');
  assert(/^\d+$/u.test(env.GITHUB_RUN_ID ?? ''), 'Missing run ID');
  assert(/^\d+$/u.test(env.GITHUB_RUN_ATTEMPT ?? ''), 'Missing run attempt');
  assert(/^[a-zA-Z0-9_-]+$/u.test(env.GITHUB_JOB ?? ''), 'Missing job ID');
  return `nvbes-ci-${kind}-${env.GITHUB_RUN_ID}-${env.GITHUB_RUN_ATTEMPT}-${env.GITHUB_JOB}`;
}

export function docker(...args) {
  const result = spawnSync('docker', args, { encoding: 'utf8', timeout: 120_000 });
  if (result.status !== 0) throw new Error(`docker ${args[0]} failed`);
  return result.stdout.trim();
}

export function serviceNetwork(name, port, network) {
  if (network) return { args: ['--network', network], host: name, port };
  return { args: ['--publish', `127.0.0.1::${port}`], host: '127.0.0.1' };
}

export function stopService(kind) {
  const name = serviceName(kind);
  const found = spawnSync('docker', ['inspect', name], { stdio: 'ignore', timeout: 10_000 });
  if (found.status === 0) docker('rm', '--force', '--volumes', name);
  else if (found.status !== 1) throw new Error('Cannot inspect test service for cleanup');
}

export async function startService(kind) {
  const name = serviceName(kind);
  const port = kind === 'postgres' ? 5432 : 6379;
  let network;
  if (existsSync('/.dockerenv')) {
    const networks = JSON.parse(
      docker('inspect', hostname(), '--format', '{{json .NetworkSettings.Networks}}'),
    );
    network = Object.keys(networks).find((value) => !['host', 'bridge', 'none'].includes(value));
    assert(network, 'Containerized runner requires a named Docker network');
  }
  const connection = serviceNetwork(name, port, network);
  const image =
    kind === 'postgres'
      ? 'postgres:17-alpine'
      : 'redis@sha256:ff02b58f971e7d7d156a1267e283fcbbeee91773b6aa36c49dac28ecfe28eadf';
  docker(
    'run',
    '--detach',
    '--name',
    name,
    '--memory',
    kind === 'postgres' ? '512m' : '128m',
    '--cpus',
    '1',
    ...connection.args,
    ...(kind === 'postgres' ? ['--env', 'POSTGRES_PASSWORD=postgres'] : []),
    image,
  );
  try {
    let ready = false;
    for (let attempt = 0; attempt < 30; attempt++) {
      const probe =
        kind === 'postgres'
          ? ['pg_isready', '--host', '127.0.0.1', '--username', 'postgres']
          : ['redis-cli', 'ping'];
      const result = spawnSync('docker', ['exec', name, ...probe], {
        stdio: 'ignore',
        timeout: 5000,
      });
      if (result.status === 0) {
        ready = true;
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
    assert(ready, 'Test service did not become ready');
    if (!network) {
      const binding = docker('port', name, `${port}/tcp`);
      assert(/^127\.0\.0\.1:\d+$/u.test(binding), 'Unexpected test service port binding');
      connection.port = Number(binding.split(':')[1]);
    }
    return { name, host: connection.host, port: connection.port };
  } catch (error) {
    stopService(kind);
    throw error;
  }
}
