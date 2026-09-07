import { spawnSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';

function docker(...args) {
  const result = spawnSync('docker', args, { stdio: 'inherit' });
  if (result.status !== 0) throw new Error(`docker ${args[0]} failed`);
}
if (process.argv[2] === 'stop') {
  const exists = spawnSync('docker', ['inspect', 'nvbes-ci-postgres'], { stdio: 'ignore' });
  if (exists.status === 0) docker('rm', '--force', 'nvbes-ci-postgres');
} else {
  const plan = JSON.parse(process.env.NVBES_CI_PLAN);
  docker(
    'run',
    '--detach',
    '--name',
    'nvbes-ci-postgres',
    '--env',
    'POSTGRES_PASSWORD=postgres',
    '--publish',
    '127.0.0.1:5432:5432',
    'postgres:17-alpine',
  );
  let ready = false;
  for (let attempt = 0; attempt < 30; attempt++) {
    if (
      // The image's temporary initialization server only listens on a Unix
      // socket. TCP readiness waits for the final server after its restart.
      spawnSync(
        'docker',
        [
          'exec',
          'nvbes-ci-postgres',
          'pg_isready',
          '--host',
          '127.0.0.1',
          '--username',
          'postgres',
        ],
        {
          stdio: 'ignore',
        },
      ).status === 0
    ) {
      ready = true;
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  if (!ready) throw new Error('PostgreSQL did not become ready');
  for (const project of plan.databaseExecution) {
    const scope = {
      'email-worker': 'email',
      'account-service': 'account',
      'billing-service': 'billing',
      'identity-service': 'identity',
      'trust-risk-service': 'trust_risk',
    }[project];
    if (!scope) throw new Error(`No isolated database for ${project}`);
    const database = `nvbes_${scope}_test`;
    docker(
      'exec',
      'nvbes-ci-postgres',
      'createdb',
      '--host',
      '127.0.0.1',
      '--username',
      'postgres',
      database,
    );
    appendFileSync(
      process.env.GITHUB_ENV,
      `NVBES_${scope.toUpperCase()}_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/${database}\n`,
    );
  }
}
