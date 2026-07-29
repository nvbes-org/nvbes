import path from 'node:path';
import { resolveConfig } from 'vite-plus';
import { describe, expect, it } from 'vite-plus/test';

const workspaceRoot = path.resolve(import.meta.dirname, '../../..');
const posthogWebConfigurations = [
  path.resolve(import.meta.dirname, '../vite.config.ts'),
  path.resolve(import.meta.dirname, '../../cloud-web/vite.config.ts'),
];

describe('PostHog web environment loading', () => {
  it.each(posthogWebConfigurations)(
    'exposes workspace VITE variables to application code: %s',
    async (configurationPath) => {
      const configuration = await resolveConfig(
        {
          configFile: configurationPath,
          root: path.dirname(configurationPath),
          mode: 'test',
          logLevel: 'silent',
        },
        'serve',
      );

      expect(configuration.envDir).toBe(workspaceRoot);
    },
  );
});
