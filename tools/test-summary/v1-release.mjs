import { execFileSync } from 'node:child_process';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { confinedRead, sha256, verifyBundle } from './v1-bundle-verification.mjs';
import { productionUnits } from './v1-catalogue.mjs';
import { evaluateV1 } from './v1-evidence.mjs';
import { validateManifest } from './v1-manifest.mjs';
import { githubArtifactReader } from './v1-ci-artifacts.mjs';

export function loadV1(cwd) {
  const rootBytes = confinedRead(cwd, 'docs/testing/v1/manifest.json');
  const root = JSON.parse(rootBytes);
  const domainBytes = Object.entries(root.domains)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([, file]) => confinedRead(cwd, file));
  const domains = domainBytes.map((bytes) => JSON.parse(bytes));
  validateManifest(root, domains);
  const manifestDigest = sha256(JSON.stringify([rootBytes.toString(), ...domainBytes.map(String)]));
  return { root, domains, manifestDigest };
}

export function renderV1(result, sha) {
  const safe = (text) => String(text).replace(/[|\r\n]/gu, ' ');
  return [
    `# V1 Test Summary — ${sha}`,
    '',
    `Verdict: ${result.verdict}`,
    '',
    '| Domain | Suite | Result |',
    '|---|---|---|',
    ...result.rows.map(
      (row) => `| ${safe(row.domain)} | ${safe(row.suite)} | ${safe(row.status)} |`,
    ),
    '',
    '## Blocking findings',
    '',
    ...result.failures.map((failure) => `- ${safe(failure)}`),
    '',
  ].join('\n');
}

function main() {
  const cwd = process.cwd();
  const context = loadV1(cwd);
  const units = productionUnits(context.root, context.domains, cwd);
  const sha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  if (process.argv.includes('--catalogue')) {
    console.log(JSON.stringify({ ...context, units }, null, 2));
    return;
  }
  const env = process.env;
  let bundle;
  let verified = false;
  let verificationError;
  try {
    if (!env.NVBES_V1_EVIDENCE_FILE) throw new Error('Evidence bundle not supplied');
    if (env.NVBES_RELEASE_SHA !== sha)
      throw new Error('Release SHA must match the checked-out commit');
    if (
      execFileSync('git', ['status', '--porcelain', '--untracked-files=normal'], {
        encoding: 'utf8',
      }).trim()
    ) {
      throw new Error('Release evidence requires a clean checkout');
    }
    const directory = path.dirname(path.resolve(env.NVBES_V1_EVIDENCE_FILE));
    bundle = verifyBundle({
      bytes: confinedRead(directory, path.basename(env.NVBES_V1_EVIDENCE_FILE)),
      signature: readFileSync(env.NVBES_V1_SIGNATURE_FILE),
      publicKey: readFileSync(env.NVBES_V1_PUBLIC_KEY_FILE),
      publicKeyDigest: env.NVBES_V1_PUBLIC_KEY_SHA256,
      artifactDirectory: directory,
      units,
      repository: 'nvbes-org/nvbes',
      workflows: ['.github/workflows/ci.yml', '.github/workflows/v1-testing.yml'],
      getCiArtifact: githubArtifactReader('nvbes-org/nvbes'),
      getRun: (runId) =>
        JSON.parse(
          execFileSync(
            'gh',
            ['api', '--hostname', 'github.com', `repos/nvbes-org/nvbes/actions/runs/${runId}`],
            {
              encoding: 'utf8',
              maxBuffer: 4 * 1024 * 1024,
              timeout: 30000,
            },
          ),
        ),
    });
    verified = true;
  } catch (error) {
    verificationError = error.message;
  }
  const result = evaluateV1({ ...context, units, bundle, expectedSha: sha, verified });
  if (verificationError) result.failures.unshift(verificationError);
  const output = path.join(cwd, '.temp/test-summary');
  mkdirSync(output, { recursive: true });
  writeFileSync(path.join(output, 'v1.json'), JSON.stringify(result, null, 2));
  writeFileSync(path.join(output, 'v1.md'), renderV1(result, sha));
  console.log(
    `${result.verdict}: ${result.failures.length} blocking findings; .temp/test-summary/v1.md`,
  );
  if (result.verdict !== 'GO') process.exitCode = 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    console.error(`NO-GO: ${error.message}`);
    process.exitCode = 1;
  }
}
