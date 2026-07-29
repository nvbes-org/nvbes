import path from 'node:path';
import { resolveConfig } from 'vite-plus';
import { describe, expect, it } from 'vite-plus/test';

const webConfigurations = [
  path.resolve(import.meta.dirname, '../vite.config.ts'),
  path.resolve(import.meta.dirname, '../../cloud-web/vite.config.ts'),
  path.resolve(import.meta.dirname, '../../enterprise-web/vite.config.ts'),
];

function expectReportOnlyIntegrityPolicy(headers: unknown): void {
  expect(headers).toEqual(
    expect.objectContaining({
      'Integrity-Policy-Report-Only': 'blocked-destinations=(script)',
    }),
  );
  expect(headers).not.toHaveProperty('Integrity-Policy');
}

describe('web Integrity-Policy headers', () => {
  it.each(webConfigurations)(
    'keeps script integrity monitoring non-blocking for code-split builds: %s',
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

      expectReportOnlyIntegrityPolicy(configuration.server.headers);
      expectReportOnlyIntegrityPolicy(configuration.preview.headers);
    },
  );
});
