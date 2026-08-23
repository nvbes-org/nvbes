import { readFileSync } from 'node:fs';
import path from 'node:path';
import { parse } from 'vite-plus';
import { describe, expect, it } from 'vite-plus/test';

const workspaceRoot = path.resolve(import.meta.dirname, '../../..');

function readJson(relativePath: string): {
  dependencies?: Record<string, string>;
} {
  return JSON.parse(readFileSync(path.join(workspaceRoot, relativePath), 'utf8'));
}

async function readStringArrayProperty(
  sourceText: string,
  expectedPropertyName: string,
): Promise<string[]> {
  const parsed = await parse('vite.config.ts', sourceText);
  const values: string[] = [];

  function visit(node: unknown): void {
    if (Array.isArray(node)) {
      for (const child of node) {
        visit(child);
      }
      return;
    }

    if (!isRecord(node)) {
      return;
    }

    if (
      node.type === 'Property' &&
      propertyName(node.key) === expectedPropertyName &&
      isRecord(node.value) &&
      node.value.type === 'ArrayExpression' &&
      Array.isArray(node.value.elements)
    ) {
      for (const element of node.value.elements) {
        if (isRecord(element) && element.type === 'Literal' && typeof element.value === 'string') {
          values.push(element.value);
        }
      }
    }

    for (const child of Object.values(node)) {
      visit(child);
    }
  }

  visit(parsed.program);
  return values;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function propertyName(node: unknown): string | undefined {
  if (!isRecord(node)) {
    return undefined;
  }

  if (node.type === 'Identifier' && typeof node.name === 'string') {
    return node.name;
  }

  return node.type === 'Literal' && typeof node.value === 'string' ? node.value : undefined;
}

describe('identity-web bundle policy', () => {
  it('uses the workspace Sentry catalog entry in every importing project', () => {
    const workspace = readFileSync(path.join(workspaceRoot, 'pnpm-workspace.yaml'), 'utf8');
    const identityWeb = readJson('apps/identity-web/package.json');
    const webRuntime = readJson('libs/ts/web-runtime/package.json');

    expect(workspace).toMatch(/^\s{2}'@sentry\/browser': \^10\.68\.0$/mu);
    expect(identityWeb.dependencies?.['@sentry/browser']).toBe('catalog:');
    expect(webRuntime.dependencies?.['@sentry/browser']).toBe('catalog:');
  });

  it('keeps bundle analysis output out of the PWA precache', async () => {
    const viteConfig = readFileSync(
      path.join(workspaceRoot, 'apps/identity-web/vite.config.ts'),
      'utf8',
    );

    expect(await readStringArrayProperty(viteConfig, 'globIgnores')).toContain('**/stats.html');
  });
});
