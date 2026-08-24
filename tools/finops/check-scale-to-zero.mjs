import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const ROOT = path.resolve(process.argv[2] ?? 'infrastructure');
const RESOURCE_TYPES = new Map([
  ['scaleway_container', 'min_scale'],
  ['scaleway_sdb_sql_database', 'min_cpu'],
]);

async function terraformFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    if (entry.name === '.terraform') continue;
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await terraformFiles(entryPath)));
    else if (entry.isFile() && entry.name.endsWith('.tf')) files.push(entryPath);
  }

  return files;
}

function resourceBlocks(source, resourceType) {
  const pattern = new RegExp(`resource\\s+"${resourceType}"\\s+"[^"]+"\\s*\\{`, 'g');
  const blocks = [];

  for (const match of source.matchAll(pattern)) {
    let depth = 1;
    let cursor = match.index + match[0].length;
    while (cursor < source.length && depth > 0) {
      if (source[cursor] === '{') depth += 1;
      if (source[cursor] === '}') depth -= 1;
      cursor += 1;
    }
    blocks.push(source.slice(match.index, cursor));
  }

  return blocks;
}

const violations = [];

for (const file of await terraformFiles(ROOT)) {
  const source = await readFile(file, 'utf8');
  for (const [resourceType, minimumAttribute] of RESOURCE_TYPES) {
    for (const block of resourceBlocks(source, resourceType)) {
      if (!new RegExp(`\\b${minimumAttribute}\\s*=\\s*0\\b`).test(block)) {
        violations.push(
          `${path.relative(process.cwd(), file)}: ${resourceType} must declare ${minimumAttribute} = 0`,
        );
      }
    }
  }
}

if (violations.length > 0) {
  console.error(violations.join('\n'));
  process.exitCode = 1;
} else {
  console.log('FinOps scale-to-zero guard passed for all Scaleway runtimes.');
}
