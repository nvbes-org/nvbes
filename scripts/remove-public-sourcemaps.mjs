import { readdir, rm } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const cwd = process.cwd();
const rawDir = process.argv[2] ?? 'dist';
const outputDirectory = path.resolve(cwd, rawDir);
const rel = path.relative(cwd, outputDirectory);
if (rel.startsWith('..') || path.isAbsolute(rel)) {
  console.error('Invalid path: output directory must stay within current directory');
  process.exit(1);
}
const sourceMaps = await findSourceMaps(outputDirectory);

await Promise.all(sourceMaps.map((sourceMap) => rm(sourceMap)));

if (sourceMaps.length > 0) {
  console.log(`Removed ${sourceMaps.length} source map(s) from ${outputDirectory}`);
}

async function findSourceMaps(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const sourceMaps = [];

  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      sourceMaps.push(...(await findSourceMaps(entryPath)));
    } else if (entry.name.endsWith('.map')) {
      sourceMaps.push(entryPath);
    }
  }

  return sourceMaps;
}
