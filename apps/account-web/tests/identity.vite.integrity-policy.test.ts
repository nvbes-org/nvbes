import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vite-plus/test';

const webConfigurations = [
  path.resolve(import.meta.dirname, '../vite.config.ts'),
  path.resolve(import.meta.dirname, '../../cloud-web/vite.config.ts'),
  path.resolve(import.meta.dirname, '../../enterprise-web/vite.config.ts'),
];

describe('web Integrity-Policy headers', () => {
  it.each(webConfigurations)(
    'keeps script integrity monitoring non-blocking for code-split builds: %s',
    (configurationPath) => {
      const configuration = fs.readFileSync(configurationPath, 'utf-8');

      expect(configuration).toContain("'Integrity-Policy-Report-Only': integrityPolicyScripts");
      expect(configuration).not.toMatch(/['"]Integrity-Policy['"]\s*:/u);
    },
  );
});
