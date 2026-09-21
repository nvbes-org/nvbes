import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import type { Plugin } from 'vite-plus';

export function sriPlugin(): Plugin {
  let outDir: string;

  return {
    name: 'vite-plugin-sri',
    enforce: 'post',
    configResolved(config) {
      outDir = config.build.outDir;
    },
    closeBundle: {
      order: 'post',
      handler() {
        injectSriFromFiles(outDir);
      },
    },
  };
}

export function injectSriFromFiles(outDir: string): void {
  const htmlPath = path.resolve(outDir, 'index.html');
  if (!fs.existsSync(htmlPath)) return;
  let html = fs.readFileSync(htmlPath, 'utf-8');

  for (const name of outputFileNames(outDir)) {
    if (!/\.(?:css|js)$/u.test(name)) continue;
    const filePath = path.resolve(outDir, name);
    const hash = crypto.createHash('sha384').update(fs.readFileSync(filePath)).digest('base64');
    const integrity = `sha384-${hash}`;
    const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    html = updateIntegrity(html, 'script', 'src', escaped, integrity);
    html = updateIntegrity(html, 'link', 'href', escaped, integrity);
  }

  fs.writeFileSync(htmlPath, html);
}

function outputFileNames(directory: string, prefix = ''): string[] {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const name = prefix ? `${prefix}/${entry.name}` : entry.name;
    return entry.isDirectory()
      ? outputFileNames(path.resolve(directory, entry.name), name)
      : [name];
  });
}

function updateIntegrity(
  html: string,
  tag: 'link' | 'script',
  attribute: 'href' | 'src',
  escapedName: string,
  integrity: string,
): string {
  return html.replace(
    new RegExp(`(<${tag}[^>]*${attribute}="[^"]*${escapedName}"[^>]*)(>)`, 'g'),
    (match) => {
      const updated = match.replace(/[ \t]+integrity="[^"]*"/g, '');
      return updated.includes('crossorigin')
        ? updated.replace('>', ` integrity="${integrity}">`)
        : updated.replace('>', ` integrity="${integrity}" crossorigin="anonymous">`);
    },
  );
}
