import { appendFileSync } from 'node:fs';
import { docker, startService, stopService } from './test-service.mjs';

if (process.argv[2] === 'stop') stopService('postgres');
else {
  const { name, host, port } = await startService('postgres');
  try {
    docker(
      'exec',
      name,
      'createdb',
      '--host',
      '127.0.0.1',
      '--username',
      'postgres',
      'nvbes_security_test',
    );
    appendFileSync(
      process.env.GITHUB_ENV,
      `NVBES_SECURITY_TEST_DATABASE_URL=postgres://postgres:postgres@${host}:${port}/nvbes_security_test\n`,
    );
  } catch (error) {
    stopService('postgres');
    throw error;
  }
}
