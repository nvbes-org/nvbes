import { execFile, spawn } from 'node:child_process';
import { randomBytes, randomUUID } from 'node:crypto';
import { createServer } from 'node:net';
import { promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';

const execute = promisify(execFile);

// Do not inherit production credentials, telemetry, proxies or dotenv settings.
export const runtimeEnvironment = () => ({
  PATH: process.env.PATH,
  HOME: process.env.HOME,
  NVBES_ENVIRONMENT: 'test',
});

export async function command(file, args, env = runtimeEnvironment()) {
  try {
    const { stdout } = await execute(file, args, { env, timeout: 60_000, maxBuffer: 1_048_576 });
    return stdout.trim();
  } catch {
    // Child errors include command arguments and potentially credentials.
    throw new Error(`${file} failed; child output suppressed to protect fixture credentials`);
  }
}

export async function unusedPort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const { port } = server.address();
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  return port;
}

export class RuntimeFixture {
  cleanups = [];
  container;

  async database() {
    const name = `nvbes-identity-runtime-test-${randomUUID()}`;
    const password = randomBytes(32).toString('hex');
    await command('docker', [
      'run',
      '--detach',
      '--rm',
      '--pull=never',
      '--name',
      name,
      '--memory=256m',
      '--publish',
      '127.0.0.1::5432',
      '--env',
      'POSTGRES_USER=identity_test',
      '--env',
      `POSTGRES_PASSWORD=${password}`,
      'postgres:17-alpine',
    ]);
    this.cleanups.push(() => command('docker', ['rm', '--force', name]));
    this.container = name;
    const binding = await command('docker', ['port', name, '5432/tcp']);
    if (!/^127\.0\.0\.1:\d+$/.test(binding)) throw new Error('Database must bind only loopback');
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt++) {
      try {
        // PostgreSQL's initialization server listens on a Unix socket only.
        // Wait for TCP so migrations cannot race its shutdown/restart.
        await command('docker', [
          'exec',
          name,
          'pg_isready',
          '-h',
          '127.0.0.1',
          '-U',
          'identity_test',
        ]);
        ready = true;
        break;
      } catch {
        await delay(100);
      }
    }
    if (!ready) throw new Error('Isolated PostgreSQL did not become ready');
    const databases = {};
    for (const service of ['identity', 'account', 'billing']) {
      const database = `nvbes_${service}_test_runtime`;
      await command('docker', [
        'exec',
        name,
        'psql',
        '-U',
        'identity_test',
        '-d',
        'postgres',
        '-v',
        'ON_ERROR_STOP=1',
        '-c',
        `CREATE DATABASE ${database}`,
      ]);
      databases[service] = `postgres://identity_test:${password}@${binding}/${database}`;
    }
    return databases;
  }

  async sql(service, statement) {
    if (!this.container || !['identity', 'account', 'billing'].includes(service)) {
      throw new Error('Only fixture-owned databases are allowed');
    }
    return command('docker', [
      'exec',
      this.container,
      'psql',
      '-U',
      'identity_test',
      '-d',
      `nvbes_${service}_test_runtime`,
      '-v',
      'ON_ERROR_STOP=1',
      '-Atc',
      statement,
    ]);
  }

  async start(file, env, origin, readiness) {
    const child = spawn(file, [], { env, stdio: ['ignore', 'ignore', 'pipe'] });
    let diagnostic = '';
    child.stderr.on('data', (chunk) => {
      // Expose only recognized static startup errors, never arbitrary runtime output.
      if (chunk.toString().includes('invalid WebAuthn RP configuration')) {
        diagnostic = ': invalid WebAuthn RP configuration';
      }
    });
    let terminal = false;
    const completion = new Promise((resolve) => {
      child.once('error', () => {
        terminal = true;
        resolve();
      });
      child.once('exit', () => {
        terminal = true;
        resolve();
      });
    });
    const stop = async () => {
      if (terminal) return;
      child.kill('SIGTERM');
      const timer = setTimeout(() => child.kill('SIGKILL'), 5000);
      try {
        await completion;
      } finally {
        clearTimeout(timer);
      }
    };
    this.cleanups.push(stop);
    for (let attempt = 0; attempt < 150; attempt++) {
      if (terminal) throw new Error(`${file} exited before readiness${diagnostic}`);
      try {
        const response = await fetch(`${origin}${readiness}`, { signal: AbortSignal.timeout(500) });
        await response.arrayBuffer();
        if (response.ok) return { stop };
      } catch {
        /* Listener may not be bound yet. */
      }
      await delay(100);
    }
    throw new Error(`${file} did not become ready`);
  }

  async close() {
    const errors = [];
    for (const cleanup of this.cleanups.reverse()) {
      try {
        await cleanup();
      } catch (error) {
        errors.push(error);
      }
    }
    if (errors.length) throw new AggregateError(errors, 'Fixture cleanup failed');
  }
}
