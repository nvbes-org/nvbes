import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import type { Plugin } from 'vite-plus';

export function sriPlugin(): Plugin {
  let outDir: string;
  const hashes = new Map<string, string>();

  return {
    name: 'vite-plugin-sri',
    enforce: 'post',
    configResolved(config) {
      outDir = config.build.outDir;
    },
    generateBundle(_opts, bundle) {
      for (const [name, info] of Object.entries(bundle)) {
        const source = info.type === 'chunk' ? info.code : info.source;
        if (!source) continue;
        const input = typeof source === 'string' ? source : Buffer.from(source).toString('utf-8');
        const hash = crypto.createHash('sha384').update(input, 'utf-8').digest('base64');
        hashes.set(name, `sha384-${hash}`);
      }
    },
    closeBundle() {
      const htmlPath = path.resolve(outDir, 'index.html');
      if (!fs.existsSync(htmlPath)) return;
      let html = fs.readFileSync(htmlPath, 'utf-8');

      for (const [name, integrity] of hashes) {
        const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        html = html.replace(
          new RegExp(`(<script[^>]*src="[^"]*${escaped}"[^>]*)(>)`, 'g'),
          (match) => {
            if (match.includes('integrity=')) return match;
            return match.includes('crossorigin')
              ? match.replace('>', ` integrity="${integrity}">`)
              : match.replace('>', ` integrity="${integrity}" crossorigin="anonymous">`);
          },
        );
        html = html.replace(
          new RegExp(`(<link[^>]*href="[^"]*${escaped}"[^>]*)(>)`, 'g'),
          (match) => {
            if (match.includes('integrity=')) return match;
            return match.includes('crossorigin')
              ? match.replace('>', ` integrity="${integrity}">`)
              : match.replace('>', ` integrity="${integrity}" crossorigin="anonymous">`);
          },
        );
      }

      fs.writeFileSync(htmlPath, html);
    },
  };
}
