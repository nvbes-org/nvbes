import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

export function deriveReleaseBlockingCategories(manifest) {
  const blocking = new Set();
  for (const contract of manifest.coverage ?? []) {
    if (contract?.execution === 'gap' || nonEmpty(contract?.limitation)) {
      blocking.add(contract.category);
    }
  }
  for (const knownGap of manifest.knownGaps ?? []) {
    if (knownGap?.releaseBlocking === true) {
      for (const category of knownGap.categories ?? []) {
        blocking.add(category);
      }
    }
  }
  return [...blocking].sort((a, b) => a.localeCompare(b));
}

export function validateAccountReleaseReadiness(manifest) {
  if (!Array.isArray(manifest?.coverage)) {
    throw new Error('Account test manifest coverage is missing');
  }
  const blockers = deriveReleaseBlockingCategories(manifest);
  const declared = [...(manifest.releaseReadiness?.blockingCategories ?? [])].sort((a, b) =>
    a.localeCompare(b),
  );
  if (JSON.stringify(declared) !== JSON.stringify(blockers)) {
    throw new Error('Account release-readiness blocker registry is stale');
  }
  const expectedStatus = blockers.length === 0 ? 'ready' : 'blocked';
  if (manifest.releaseReadiness?.status !== expectedStatus) {
    throw new Error(`Account release-readiness status must be ${expectedStatus}`);
  }
  if (blockers.length > 0) {
    throw new Error(
      `Account production release is blocked by incomplete test categories: ${blockers.join(', ')}`,
    );
  }
  return { categories: manifest.coverage.length };
}

function nonEmpty(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

async function main() {
  const manifest = JSON.parse(await readFile('docs/testing/account-test-manifest.json', 'utf8'));
  const result = validateAccountReleaseReadiness(manifest);
  process.stdout.write(
    `Account production test readiness passed for ${result.categories} categories.\n`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(
      `${error instanceof Error ? error.message : 'Account release readiness failed'}\n`,
    );
    process.exitCode = 1;
  });
}
