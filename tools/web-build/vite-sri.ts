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
    html = updateIntegrity(html, 'script', 'src', name, integrity);
    html = updateIntegrity(html, 'link', 'href', name, integrity);
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
  name: string,
  integrity: string,
): string {
  const token = `<${tag}`;
  const attrLiteral = `${attribute}="`;
  let result = '';
  let searchFrom = 0;

  while (searchFrom < html.length) {
    const start = html.indexOf(token, searchFrom);
    if (start === -1) {
      result += html.slice(searchFrom);
      break;
    }
    const close = html.indexOf('>', start + token.length);
    if (close === -1) {
      throw new Error(`Malformed <${tag}> element in built index.html`);
    }
    result += html.slice(searchFrom, start);
    let tagText = html.slice(start, close + 1);

    const attrAt = tagText.indexOf(attrLiteral, token.length);
    const valueEnd = attrAt === -1 ? -1 : tagText.indexOf('"', attrAt + attrLiteral.length);
    if (attrAt !== -1 && valueEnd !== -1) {
      const value = tagText.slice(attrAt + attrLiteral.length, valueEnd);
      if (value.endsWith(name)) {
        tagText = stripAttribute(tagText, 'integrity');
        const base = tagText.slice(0, -1);
        tagText = tagText.includes('crossorigin')
          ? `${base} integrity="${integrity}">`
          : `${base} integrity="${integrity}" crossorigin="anonymous">`;
      }
    }

    result += tagText;
    searchFrom = close + 1;
  }

  return result;
}

function stripAttribute(tagText: string, name: string): string {
  const literal = `${name}="`;
  let at = tagText.indexOf(literal);
  while (at !== -1) {
    const valueEnd = tagText.indexOf('"', at + literal.length);
    if (valueEnd === -1) break;
    let cutStart = at;
    while (
      cutStart > 0 &&
      (tagText.charCodeAt(cutStart - 1) === 32 || tagText.charCodeAt(cutStart - 1) === 9)
    ) {
      cutStart -= 1;
    }
    tagText = tagText.slice(0, cutStart) + tagText.slice(valueEnd + 1);
    at = tagText.indexOf(literal, cutStart);
  }
  return tagText;
}
