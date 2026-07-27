import { readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vite-plus/test';

const posthogWebConfigurations = [
  path.resolve(import.meta.dirname, '../vite.config.ts'),
  path.resolve(import.meta.dirname, '../../cloud-web/vite.config.ts'),
];

describe('PostHog web environment loading', () => {
  it.each(posthogWebConfigurations)(
    'exposes workspace VITE variables to application code: %s',
    (configurationPath) => {
      const configuration = readFileSync(configurationPath, 'utf8');

      expect(configuration).toContain("const workspaceRoot = path.resolve(__dirname, '../..');");
      expect(configuration).toContain("const rootEnv = loadEnv(mode, workspaceRoot, '');");
      expect(configuration).toContain('envDir: workspaceRoot,');
    },
  );
});
