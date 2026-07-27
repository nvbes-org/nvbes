type EnvironmentSource = Record<string, string | undefined>;

const FARO_URL_KEYS = [
  'VITE_FARO_URL_ACCOUNT_WEB',
  'VITE_FARO_URL',
  'VITE_GRAFANA_FARO_URL',
] as const;

export function resolveFaroUrl(envSources: readonly EnvironmentSource[]): string {
  const url = firstConfiguredValue(envSources, FARO_URL_KEYS);
  const required = firstConfiguredValue(envSources, ['NVBES_REQUIRE_FARO_ACCOUNT_WEB']) === 'true';

  if (required && !url) {
    throw new Error(
      'Faro is required for account-web, but VITE_FARO_URL_ACCOUNT_WEB is missing. ' +
        'Create the Grafana Frontend Observability application and configure its collector URL.',
    );
  }

  return url;
}

function firstConfiguredValue(
  envSources: readonly EnvironmentSource[],
  keys: readonly string[],
): string {
  for (const source of envSources) {
    for (const key of keys) {
      const value = source[key]?.trim();
      if (value) {
        return value;
      }
    }
  }

  return '';
}
