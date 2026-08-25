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

function sanitizeHcl(source) {
  const output = [...source];
  let inString = false;
  let escaped = false;
  let inBlockComment = false;
  let inLineComment = false;
  let heredoc = null;
  let lineStart = 0;

  const mask = (index) => {
    if (output[index] !== '\n' && output[index] !== '\r') output[index] = ' ';
  };
  const maskRange = (start, end) => {
    for (let index = start; index < end; index += 1) mask(index);
  };
  const lineEnd = (start) => {
    const end = source.indexOf('\n', start);
    return end === -1 ? source.length : end;
  };

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    const nextCharacter = source[index + 1];

    if (heredoc !== null) {
      if (character === '\n' || character === '\r') {
        lineStart = index + 1;
        continue;
      }
      const end = lineEnd(index);
      const line = source.slice(lineStart, end).replace(/\r$/, '');
      const terminator = heredoc.indented
        ? new RegExp(`^[\\t ]*${heredoc.label}[\\t ]*$`).test(line)
        : line === heredoc.label;
      maskRange(index, end);
      index = end - 1;
      if (terminator) heredoc = null;
      continue;
    }

    if (inLineComment) {
      mask(index);
      if (character === '\n' || character === '\r') inLineComment = false;
      continue;
    }
    if (inBlockComment) {
      mask(index);
      if (character === '*' && nextCharacter === '/') {
        mask(index + 1);
        index += 1;
        inBlockComment = false;
      }
      continue;
    }
    if (inString) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') inString = false;
      continue;
    }

    if (character === '\n' || character === '\r') {
      lineStart = index + 1;
    } else if (character === '"') {
      inString = true;
    } else if (character === '#') {
      mask(index);
      inLineComment = true;
    } else if (character === '/' && nextCharacter === '/') {
      mask(index);
      mask(index + 1);
      index += 1;
      inLineComment = true;
    } else if (character === '/' && nextCharacter === '*') {
      mask(index);
      mask(index + 1);
      index += 1;
      inBlockComment = true;
    } else if (character === '<' && nextCharacter === '<') {
      const header = source.slice(index, lineEnd(index));
      const match = header.match(/^<<(-?)([A-Za-z_][A-Za-z0-9_-]*)/);
      if (match !== null) {
        maskRange(index, lineEnd(index));
        heredoc = { indented: match[1] === '-', label: match[2] };
        index = lineEnd(index) - 1;
      }
    }
  }

  return output.join('');
}

function resourceBlocks(source, resourceType) {
  const pattern = new RegExp(`^[\\t ]*resource\\s+"${resourceType}"\\s+"[^"]+"\\s*\\{`, 'gm');
  const blocks = [];

  for (const match of source.matchAll(pattern)) {
    let depth = 1;
    let cursor = match.index + match[0].length;
    let inString = false;
    let escaped = false;
    while (cursor < source.length && depth > 0) {
      const character = source[cursor];
      if (inString) {
        if (escaped) escaped = false;
        else if (character === '\\') escaped = true;
        else if (character === '"') inString = false;
      } else if (character === '"') {
        inString = true;
      } else if (character === '{') {
        depth += 1;
      } else if (character === '}') {
        depth -= 1;
      }
      cursor += 1;
    }
    blocks.push(source.slice(match.index, cursor));
  }

  return blocks;
}

function literalInteger(block, attribute) {
  let depth = 0;
  let inString = false;
  let escaped = false;
  let directSource = '';
  for (const character of block) {
    if (inString) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') inString = false;
      directSource += ' ';
    } else if (character === '"') {
      inString = true;
      directSource += ' ';
    } else {
      if (character === '{') depth += 1;
      else if (character === '}') depth -= 1;
      directSource += depth === 1 || character === '\n' || character === '\r' ? character : ' ';
    }
  }
  const pattern = new RegExp(`^[\\t ]*${attribute}[\\t ]*=[\\t ]*([01])[\\t ]*(?:\\r)?$`, 'm');
  const match = directSource.match(pattern);
  return match === null ? undefined : match[1];
}

const violations = [];

for (const file of (await terraformFiles(ROOT)).sort()) {
  const source = await readFile(file, 'utf8');
  const sanitizedSource = sanitizeHcl(source);
  for (const [resourceType, policy] of RESOURCE_POLICIES) {
    for (const block of resourceBlocks(sanitizedSource, resourceType)) {
      if (literalInteger(block, policy.minimumAttribute) !== '0') {
        violations.push(
          `${path.relative(process.cwd(), file)}: ${resourceType} must declare ${policy.minimumAttribute} = 0`,
        );
      }
      const maximum = literalInteger(block, policy.maximumAttribute);
      if (maximum === undefined) {
        violations.push(
          `${path.relative(process.cwd(), file)}: ${resourceType} must declare ${policy.maximumAttribute} as an integer <= 1`,
        );
      }
    }
  }
}

if (violations.length > 0) {
  console.error(violations.sort().join('\n'));
  process.exitCode = 1;
} else {
  console.log('FinOps scale bounds passed for all Scaleway runtimes.');
}
