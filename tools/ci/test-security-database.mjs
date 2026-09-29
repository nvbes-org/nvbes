import { appendFileSync } from 'node:fs';
import { docker, startService, stopService } from './test-service.mjs';

/**
 * `nvbes_security_test` heberge les pools a schema isole de nvbes-test-utils.
 * `nvbes_coverage_test` sert de base mere aux tests `#[sqlx::test]`, qui creent
 * puis detruisent une base par test ; sans elle la couverture workspace ne peut
 * pas mesurer les couches d'acces PostgreSQL.
 */
const DATABASES = ['nvbes_security_test', 'nvbes_coverage_test'];

if (process.argv[2] === 'stop') stopService('postgres');
else {
  const { name, host, port } = await startService('postgres');
  try {
    for (const database of DATABASES) {
      docker('exec', name, 'createdb', '--host', '127.0.0.1', '--username', 'postgres', database);
    }
    const base = `postgres://postgres:postgres@${host}:${port}`;
    appendFileSync(
      process.env.GITHUB_ENV,
      `NVBES_SECURITY_TEST_DATABASE_URL=${base}/nvbes_security_test\n` +
        `DATABASE_URL=${base}/nvbes_coverage_test\n`,
    );
  } catch (error) {
    stopService('postgres');
    throw error;
  }
}
