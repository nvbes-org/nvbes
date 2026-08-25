import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const ROOT = path.resolve(process.argv[2] ?? 'infrastructure');
const RESOURCE_POLICIES = new Map([
  [
    'scaleway_container',
    {
      minimumAttribute: 'min_scale',
      maximumAttribute: 'max_scale',
    },
  ],
  [
    'scaleway_sdb_sql_database',
    {
      minimumAttribute: 'min_cpu',
      maximumAttribute: 'max_cpu',
    },
  ],
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

function literalInteger(block, attribute) {
  const pattern = new RegExp(
    `^[\\t ]*${attribute}[\\t ]*=[\\t ]*([+-]?\\d+)[\\t ]*(?:(?:#|//).*)?(?:\\r)?$`,
    'm',
  );
  const match = block.match(pattern);
  return match === null ? undefined : Number(match[1]);
}

const violations = [];

for (const file of await terraformFiles(ROOT)) {
  const source = await readFile(file, 'utf8');
  for (const [resourceType, policy] of RESOURCE_POLICIES) {
    for (const block of resourceBlocks(source, resourceType)) {
      if (literalInteger(block, policy.minimumAttribute) !== 0) {
        violations.push(
          `${path.relative(process.cwd(), file)}: ${resourceType} must declare ${policy.minimumAttribute} = 0`,
        );
      }
      const maximum = literalInteger(block, policy.maximumAttribute);
      if (maximum === undefined || maximum > 1) {
        violations.push(
          `${path.relative(process.cwd(), file)}: ${resourceType} must declare ${policy.maximumAttribute} as an integer <= 1`,
        );
      }
    }
  }
}

if (violations.length > 0) {
  console.error(violations.join('\n'));
  process.exitCode = 1;
} else {
  console.log('FinOps scale bounds passed for all Scaleway runtimes.');
}
