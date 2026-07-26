import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vite-plus/test';
import { injectSriFromFiles } from '../../../tools/web-build/vite-sri';

let temporaryDirectory: string | null = null;

afterEach(() => {
  if (temporaryDirectory) {
    fs.rmSync(temporaryDirectory, { force: true, recursive: true });
    temporaryDirectory = null;
  }
});

describe('injectSriFromFiles', () => {
  it('hashes the final bytes written to disk and replaces stale integrity values', () => {
    temporaryDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'nvbes-sri-'));
    const assetsDirectory = path.join(temporaryDirectory, 'assets');
    fs.mkdirSync(assetsDirectory);
    fs.writeFileSync(path.join(assetsDirectory, 'app.js'), 'console.log("final");');
    fs.writeFileSync(path.join(assetsDirectory, 'app.css'), 'body{color:navy}');
    fs.writeFileSync(
      path.join(temporaryDirectory, 'index.html'),
      [
        '<script type="module" src="/assets/app.js" integrity="sha384-stale"></script>',
        '<link rel="stylesheet" href="/assets/app.css">',
      ].join('\n'),
    );

    injectSriFromFiles(temporaryDirectory);

    const html = fs.readFileSync(path.join(temporaryDirectory, 'index.html'), 'utf-8');
    expect(html).toContain(`integrity="${integrity('console.log("final");')}"`);
    expect(html).toContain(`integrity="${integrity('body{color:navy}')}"`);
    expect(html).not.toContain('sha384-stale');
  });
});

function integrity(content: string): string {
  return `sha384-${crypto.createHash('sha384').update(content).digest('base64')}`;
}
