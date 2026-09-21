import { constants } from 'node:fs';
import { chmod, copyFile, lstat, mkdir, readdir, readFile, realpath } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { REQUIRED_HUMAN_CATEGORIES } from './verify-acceptance-evidence.mjs';

const MANIFEST_NAME = 'acceptance-evidence.json';
const MANIFEST_SIGNATURE_NAME = 'acceptance-evidence.sig';
const MAX_ENTRIES = 64;
const MAX_MANIFEST_BYTES = 1024 * 1024;
const MAX_TOTAL_BYTES = 100 * 1024 * 1024;

export async function prepareAccountAcceptancePublication({ sourceRoot, publicationRoot }) {
  const canonicalSourceRoot = await requireDirectory(sourceRoot, 'source evidence');
  await requireAbsent(publicationRoot);
  const manifestPath = path.join(canonicalSourceRoot, MANIFEST_NAME);
  const manifestMetadata = await requireRegularFile(
    manifestPath,
    canonicalSourceRoot,
    MAX_MANIFEST_BYTES,
    'acceptance manifest',
  );
  const manifest = parseManifest(await readFile(manifestMetadata.path));
  const allowedFiles = publicationFiles(manifest);
  const observed = await walkEvidenceTree(canonicalSourceRoot);
  assertExactTree(observed, allowedFiles);

  await mkdir(publicationRoot, { mode: 0o700 });
  for (const relativePath of [...allowedFiles].sort((a, b) => a.localeCompare(b))) {
    const sourcePath = path.join(canonicalSourceRoot, relativePath);
    const destinationPath = path.join(publicationRoot, relativePath);
    const metadata = observed.files.get(relativePath);
    await mkdir(path.dirname(destinationPath), {
      mode: 0o700,
      recursive: true,
    });
    await copyFile(sourcePath, destinationPath, constants.COPYFILE_EXCL);
    await chmod(destinationPath, 0o600);
    if (metadata.size <= 0) {
      throw new Error(`${relativePath} must not be empty`);
    }
  }
  return {
    files: allowedFiles.size,
    totalBytes: observed.totalBytes,
  };
}

function publicationFiles(manifest) {
  assert(
    Array.isArray(manifest.reports) && manifest.reports.length === REQUIRED_HUMAN_CATEGORIES.length,
    'acceptance manifest must reference exactly 12 reports',
  );
  assert(isRecord(manifest.deployment), 'acceptance manifest deployment reference is required');
  const allowed = new Set([MANIFEST_NAME, MANIFEST_SIGNATURE_NAME]);
  const categories = new Set();
  for (const report of manifest.reports) {
    assert(isRecord(report), 'acceptance report reference must be an object');
    assert(
      REQUIRED_HUMAN_CATEGORIES.includes(report.category),
      `unexpected acceptance report ${String(report.category)}`,
    );
    assert(!categories.has(report.category), `duplicate acceptance report ${report.category}`);
    categories.add(report.category);
    addUniquePath(allowed, report.path, `${report.category} report`);
  }
  addUniquePath(allowed, manifest.deployment.path, 'deployment attestation');
  addUniquePath(allowed, manifest.deployment.signaturePath, 'deployment attestation signature');
  return allowed;
}

async function walkEvidenceTree(root) {
  const files = new Map();
  const directories = new Set();
  let entries = 0;
  let totalBytes = 0;

  async function visit(relativeDirectory, depth) {
    assert(depth <= 8, 'acceptance evidence nesting is too deep');
    const directory = path.join(root, relativeDirectory);
    for (const name of await readdir(directory)) {
      entries += 1;
      assert(entries <= MAX_ENTRIES, 'acceptance evidence has too many entries');
      assert(
        name !== '' && !name.includes('\\') && !hasControlCharacter(name),
        'acceptance evidence contains an invalid entry name',
      );
      const relativePath = path.posix.join(relativeDirectory, name);
      const absolutePath = path.join(root, relativePath);
      const metadata = await lstat(absolutePath);
      assert(!metadata.isSymbolicLink(), `${relativePath} must not be a symbolic link`);
      if (metadata.isDirectory()) {
        directories.add(relativePath);
        await visit(relativePath, depth + 1);
      } else {
        assert(metadata.isFile(), `${relativePath} must be a regular file`);
        assert(metadata.size > 0, `${relativePath} must not be empty`);
        totalBytes += metadata.size;
        assert(totalBytes <= MAX_TOTAL_BYTES, 'acceptance evidence exceeds the total size limit');
        const canonicalPath = await realpath(absolutePath);
        assert(
          canonicalPath.startsWith(`${root}${path.sep}`),
          `${relativePath} resolves outside the evidence root`,
        );
        files.set(relativePath, { path: canonicalPath, size: metadata.size });
      }
    }
  }

  await visit('', 0);
  return { directories, files, totalBytes };
}

function assertExactTree(observed, allowedFiles) {
  const observedFiles = [...observed.files.keys()].sort((a, b) => a.localeCompare(b));
  const expectedFiles = [...allowedFiles].sort((a, b) => a.localeCompare(b));
  assert(
    observedFiles.length === expectedFiles.length &&
      observedFiles.every((value, index) => value === expectedFiles[index]),
    'acceptance evidence contains missing or extra files',
  );
  const expectedDirectories = new Set();
  for (const file of allowedFiles) {
    let parent = path.posix.dirname(file);
    while (parent !== '.') {
      expectedDirectories.add(parent);
      parent = path.posix.dirname(parent);
    }
  }
  const observedDirectories = [...observed.directories].sort((a, b) => a.localeCompare(b));
  const allowedDirectories = [...expectedDirectories].sort((a, b) => a.localeCompare(b));
  assert(
    observedDirectories.length === allowedDirectories.length &&
      observedDirectories.every((value, index) => value === allowedDirectories[index]),
    'acceptance evidence contains missing or extra directories',
  );
}

function addUniquePath(paths, candidate, label) {
  const normalized = safeRelativePath(candidate, label);
  assert(!paths.has(normalized), `${label} path is duplicated`);
  paths.add(normalized);
}

function safeRelativePath(candidate, label) {
  assert(
    typeof candidate === 'string' &&
      candidate !== '' &&
      candidate === candidate.trim() &&
      !candidate.includes('\\') &&
      !hasControlCharacter(candidate),
    `${label} path is invalid`,
  );
  const normalized = path.posix.normalize(candidate);
  assert(
    normalized === candidate &&
      !path.posix.isAbsolute(candidate) &&
      !candidate.startsWith('../') &&
      candidate !== '..' &&
      candidate.split('/').length <= 8 &&
      candidate.length <= 512,
    `${label} path escapes the evidence root`,
  );
  return normalized;
}

async function requireDirectory(directoryPath, label) {
  const resolved = path.resolve(directoryPath);
  const parent = path.dirname(resolved);
  const rel = path.relative(parent, resolved);
  if (rel.startsWith('..') || path.isAbsolute(rel)) {
    throw new Error(`${label} escapes parent`);
  }
  const metadata = await lstat(resolved);
  assert(metadata.isDirectory() && !metadata.isSymbolicLink(), `${label} must be a directory`);
  return realpath(resolved);
}

async function requireRegularFile(filePath, root, maxBytes, label) {
  const resolved = path.resolve(root, filePath);
  const rel = path.relative(root, resolved);
  if (rel.startsWith('..') || path.isAbsolute(rel)) {
    throw new Error(`${label} escapes the evidence root`);
  }
  const metadata = await lstat(resolved);
  assert(metadata.isFile() && !metadata.isSymbolicLink(), `${label} must be a regular file`);
  assert(metadata.size > 0 && metadata.size <= maxBytes, `${label} has an invalid size`);
  const canonicalPath = await realpath(resolved);
  return { path: canonicalPath, size: metadata.size };
}

async function requireAbsent(candidate) {
  const resolved = path.resolve(candidate);
  const parent = path.dirname(resolved);
  const rel = path.relative(parent, resolved);
  if (rel.startsWith('..') || path.isAbsolute(rel)) {
    throw new Error('candidate escapes parent');
  }
  try {
    await lstat(resolved);
    throw new Error('acceptance publication directory must not already exist');
  } catch (error) {
    if (error?.code !== 'ENOENT') {
      throw error;
    }
  }
}

function parseManifest(content) {
  try {
    const manifest = JSON.parse(content.toString('utf8'));
    assert(isRecord(manifest), 'acceptance manifest must be an object');
    return manifest;
  } catch (error) {
    if (error instanceof SyntaxError) {
      throw new Error('acceptance manifest is not valid JSON');
    }
    throw error;
  }
}

function hasControlCharacter(value) {
  return [...value].some((character) => {
    const codePoint = character.codePointAt(0);
    return codePoint <= 0x1f || codePoint === 0x7f;
  });
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function main() {
  const [, , rawSource, rawPub] = process.argv;
  if (!rawSource || !rawPub) {
    throw new Error('usage: prepare-account-acceptance-publication.mjs <source> <publication>');
  }
  const cwd = process.cwd();
  const sourceRoot = path.resolve(cwd, rawSource);
  const relSource = path.relative(cwd, sourceRoot);
  if (relSource.startsWith('..') || path.isAbsolute(relSource)) {
    throw new Error('sourceRoot must stay within the workspace directory');
  }
  const publicationRoot = path.resolve(cwd, rawPub);
  const relPub = path.relative(cwd, publicationRoot);
  if (relPub.startsWith('..') || path.isAbsolute(relPub)) {
    throw new Error('publicationRoot must stay within the workspace directory');
  }
  const result = await prepareAccountAcceptancePublication({
    publicationRoot,
    sourceRoot,
  });
  process.stdout.write(
    `Prepared ${result.files} signed acceptance files (${result.totalBytes} bytes).\n`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'acceptance publication failed'}\n`,
    );
    process.exitCode = 1;
  });
}
