import { readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vite-plus/test';

const workspaceRoot = path.resolve(import.meta.dirname, '../../..');

function readJson(relativePath: string): {
  dependencies?: Record<string, string>;
} {
  return JSON.parse(readFileSync(path.join(workspaceRoot, relativePath), 'utf8'));
}

describe('account-web bundle policy', () => {
  it('uses the workspace Sentry catalog entry in every importing project', () => {
    const workspace = readFileSync(path.join(workspaceRoot, 'pnpm-workspace.yaml'), 'utf8');
    const accountWeb = readJson('apps/account-web/package.json');
    const webRuntime = readJson('libs/ts/web-runtime/package.json');

    expect(workspace).toMatch(/^\s{2}'@sentry\/browser': \^10\.68\.0$/mu);
    expect(accountWeb.dependencies?.['@sentry/browser']).toBe('catalog:');
    expect(webRuntime.dependencies?.['@sentry/browser']).toBe('catalog:');
  });

  it('keeps bundle analysis output out of the PWA precache', () => {
    const viteConfig = readFileSync(
      path.join(workspaceRoot, 'apps/account-web/vite.config.ts'),
      'utf8',
    );

    expect(viteConfig).toContain("'**/stats.html'");
  });
});
