import { serviceName } from '../../tools/ci/test-service.mjs';

export function isCurrentCiTestDatabaseHost(hostname, environment) {
  const targetEnvironment = (environment.NVBES_ENVIRONMENT ?? environment.NVBES_ENV)
    ?.trim()
    .toLowerCase();
  if (environment.GITHUB_ACTIONS !== 'true' || targetEnvironment !== 'ci') return false;
  try {
    return hostname === serviceName('postgres', environment);
  } catch {
    return false;
  }
}
