import { appendFileSync } from 'node:fs';
import { docker, startService, stopService } from './test-service.mjs';

if (process.argv[2] === 'stop') stopService('postgres');
else {
  const plan = JSON.parse(process.env.NVBES_CI_PLAN);
  const { name, host, port } = await startService('postgres');
  try {
    for (const project of plan.databaseExecution) {
      const scope = {
        'email-worker': 'email',
        'account-service': 'account',
        'billing-service': 'billing',
        'identity-service': 'identity',
        'trust-risk-service': 'trust_risk',
        'platform-operations-service': 'platform_operations',
      }[project];
      if (!scope) throw new Error(`No isolated database for ${project}`);
      const database = `nvbes_${scope}_test`;
      docker('exec', name, 'createdb', '--host', '127.0.0.1', '--username', 'postgres', database);
      appendFileSync(
        process.env.GITHUB_ENV,
        `NVBES_${scope.toUpperCase()}_DATABASE_URL=postgres://postgres:postgres@${host}:${port}/${database}\n`,
      );
    }
  } catch (error) {
    stopService('postgres');
    throw error;
  }
}
