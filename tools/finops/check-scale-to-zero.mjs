import { lstat, readdir, readFile, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { parse } from '@cdktn/hcl2json';

const rawRoot = process.argv[2] ?? 'infrastructure';
const resolvedRoot = path.resolve(rawRoot);
const relCwd = path.relative(process.cwd(), resolvedRoot);
const relTmp = path.relative(tmpdir(), resolvedRoot);
const isInsideCwd = !relCwd.startsWith('..') && !path.isAbsolute(relCwd);
const isInsideTmp = !relTmp.startsWith('..') && !path.isAbsolute(relTmp);
if (!isInsideCwd && !isInsideTmp) {
  console.error('Invalid path: FinOps root must be within the workspace or temporary directory');
  process.exit(1);
}
const ROOT = resolvedRoot;
const RESOURCE_POLICIES = new Map([
  ['scaleway_container', { minimumAttribute: 'min_scale', maximumAttribute: 'max_scale' }],
  ['scaleway_sdb_sql_database', { minimumAttribute: 'min_cpu', maximumAttribute: 'max_cpu' }],
]);

async function terraformFiles(directory, violations) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    if (entry.name === '.terraform') continue;
    const entryPath = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) {
      violations.push(
        `${path.relative(process.cwd(), entryPath)}: symbolic links are not supported by the FinOps gate`,
      );
      continue;
    }
    if (entry.isDirectory()) {
      files.push(...(await terraformFiles(entryPath, violations)));
    } else if (entry.isFile() && (entry.name.endsWith('.tf') || entry.name.endsWith('.tf.json'))) {
      files.push(entryPath);
    }
  }

  return files;
}

function isUnsignedZero(value) {
  return typeof value === 'number' && value === 0 && !Object.is(value, -0);
}

function isBoundedMaximum(value) {
  return typeof value === 'number' && !Object.is(value, -0) && (value === 0 || value === 1);
}

function violation(file, resourceType, message) {
  return `${path.relative(process.cwd(), file)}: ${resourceType} ${message}`;
}

function moduleViolation(file, moduleName, message) {
  return `${path.relative(process.cwd(), file)}: module ${moduleName} ${message}`;
}

function isInsideRoot(candidate) {
  const relative = path.relative(ROOT, candidate);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..');
}

function isExcludedFromScan(candidate) {
  return path.relative(ROOT, candidate).split(path.sep).includes('.terraform');
}

function isLiteralLocalModuleSource(source) {
  return (
    typeof source === 'string' &&
    (source.startsWith('./') || source.startsWith('../')) &&
    !source.includes('${') &&
    !source.includes('%{')
  );
}

async function isNonSymbolicDirectory(candidate) {
  try {
    const metadata = await lstat(candidate);
    return metadata.isDirectory() && !metadata.isSymbolicLink();
  } catch {
    return false;
  }
}

async function isExistingDirectory(candidate) {
  try {
    return (await stat(candidate)).isDirectory();
  } catch {
    return false;
  }
}

const violations = [];
const rootIsValid = await isNonSymbolicDirectory(ROOT);
if (!rootIsValid) {
  violations.push(
    `${path.relative(process.cwd(), ROOT)}: FinOps root must be an existing non-symbolic directory`,
  );
}

const files = rootIsValid ? await terraformFiles(ROOT, violations) : [];
for (const file of files.sort((a, b) => a.localeCompare(b))) {
  if (file.endsWith('.tf.json')) {
    violations.push(
      `${path.relative(process.cwd(), file)}: Terraform JSON syntax is not supported by the FinOps gate`,
    );
    continue;
  }

  let parsed;
  try {
    parsed = await parse(file, await readFile(file, 'utf8'));
  } catch {
    violations.push(`${path.relative(process.cwd(), file)}: failed to parse Terraform HCL`);
    continue;
  }

  for (const [moduleName, bodies] of Object.entries(parsed.module ?? {})) {
    for (const body of Array.isArray(bodies) ? bodies : []) {
      const source = body.source;
      if (!isLiteralLocalModuleSource(source)) {
        violations.push(
          moduleViolation(
            file,
            moduleName,
            'source must be a literal local path starting with ./ or ../',
          ),
        );
        continue;
      }
      const modulePath = path.resolve(path.dirname(file), source);
      if (!isInsideRoot(modulePath)) {
        violations.push(
          moduleViolation(file, moduleName, 'source resolves outside the FinOps root'),
        );
        continue;
      }
      if (isExcludedFromScan(modulePath)) {
        violations.push(
          moduleViolation(file, moduleName, 'source directory is excluded from the FinOps scan'),
        );
        continue;
      }
      if (!(await isExistingDirectory(modulePath))) {
        violations.push(
          moduleViolation(file, moduleName, 'source must resolve to an existing directory'),
        );
      }
    }
  }

  for (const [resourceType, policy] of RESOURCE_POLICIES) {
    const resources = parsed.resource?.[resourceType];
    if (resources === undefined || typeof resources !== 'object') continue;
    for (const bodies of Object.values(resources)) {
      for (const body of Array.isArray(bodies) ? bodies : []) {
        if (!isUnsignedZero(body[policy.minimumAttribute])) {
          violations.push(
            violation(file, resourceType, `must declare ${policy.minimumAttribute} = 0`),
          );
        }
        if (!isBoundedMaximum(body[policy.maximumAttribute])) {
          violations.push(
            violation(
              file,
              resourceType,
              `must declare ${policy.maximumAttribute} as an integer <= 1`,
            ),
          );
        }
      }
    }
  }
}

if (violations.length > 0) {
  console.error(violations.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0)).join('\n'));
  process.exitCode = 1;
} else {
  console.log('FinOps scale bounds passed for all Scaleway runtimes.');
}
