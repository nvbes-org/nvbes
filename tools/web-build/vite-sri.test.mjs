import assert from 'node:assert/strict';
import { createHash, randomBytes } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { injectSriFromFiles } from './vite-sri.ts';

function sha384(path) {
  const hash = createHash('sha384').update(readFileSync(path)).digest('base64');
  return `sha384-${hash}`;
}

function fixture() {
  const dir = mkdtempSync(join(tmpdir(), 'vite-sri-'));
  const assets = join(dir, 'assets');
  mkdirSync(assets);
  writeFileSync(
    join(dir, 'index.html'),
    [
      '<!doctype html>',
      '<html><body>',
      '<script type="module" crossorigin src="/assets/app-a1b2.js"></script>',
      '<script type="module" crossorigin src="/assets/other-zzz.ipynb"></script>',
      '<link rel="stylesheet" crossorigin href="/assets/styles-c3d4.css">',
      '</body></html>',
    ].join('\n'),
  );
  writeFileSync(join(assets, 'app-a1b2.js'), randomBytes(32));
  writeFileSync(join(assets, 'other-zzz.ipynb'), randomBytes(32));
  writeFileSync(join(assets, 'styles-c3d4.css'), randomBytes(32));
  return dir;
}

void test('injects sha384 integrity on matching script and link tags', () => {
  const dir = fixture();
  try {
    injectSriFromFiles(dir);
    const html = readFileSync(join(dir, 'index.html'), 'utf8');

    const scriptIntegrity = sha384(join(dir, 'assets', 'app-a1b2.js'));
    assert.ok(html.includes('<script type="module" crossorigin '));
    assert.ok(html.includes(`integrity="${scriptIntegrity}"`));
    assert.ok(html.includes('src="/assets/app-a1b2.js"'));

    const linkIntegrity = sha384(join(dir, 'assets', 'styles-c3d4.css'));
    assert.ok(html.includes(`integrity="${linkIntegrity}"`));

    assert.ok(html.includes('src="/assets/other-zzz.ipynb"'));
    assert.ok(!html.includes('integrity="sha384-000'));
    assert.match(html, />\s*<\/html>/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

void test('is idempotent: existing integrity is replaced, not duplicated', () => {
  const dir = fixture();
  try {
    injectSriFromFiles(dir);
    injectSriFromFiles(dir);
    const html = readFileSync(join(dir, 'index.html'), 'utf8');
    const scriptIntegrity = sha384(join(dir, 'assets', 'app-a1b2.js'));
    assert.equal(html.match(/integrity="sha384-/g)?.length ?? 0, 2);
    assert.ok(html.includes(`integrity="${scriptIntegrity}"`));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

void test('injectSriFromFiles ignores a missing index.html', () => {
  const dir = mkdtempSync(join(tmpdir(), 'vite-sri-empty-'));
  try {
    assert.doesNotThrow(() => injectSriFromFiles(dir));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
