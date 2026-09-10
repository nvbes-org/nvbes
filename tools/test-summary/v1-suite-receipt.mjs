import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, renameSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { loadV1 } from './v1-release.mjs';

const sha256 = (value) => createHash('sha256').update(value).digest('hex');
const canonicalDate = (value) => new Date(value).toISOString();

export function ciContext(env) {
  assert(/^[a-f0-9]{40}$/u.test(env.GITHUB_SHA ?? ''), 'Full GitHub candidate SHA required');
  assert(/^[1-9][0-9]*$/u.test(env.GITHUB_RUN_ID ?? ''), 'GitHub run ID required');
  assert(
    Number.isSafeInteger(Number(env.GITHUB_RUN_ID)),
    'GitHub run ID is not safely representable',
  );
  assert.equal(env.GITHUB_RUN_ATTEMPT, '1', 'Rerun cannot produce release evidence');
  assert.equal(env.GITHUB_REPOSITORY, 'nvbes-org/nvbes', 'Wrong GitHub repository');
  assert(['push', 'workflow_dispatch'].includes(env.GITHUB_EVENT_NAME), 'Untrusted GitHub event');
  const prefix = `${env.GITHUB_REPOSITORY}/`;
  assert(env.GITHUB_WORKFLOW_REF?.startsWith(prefix), 'GitHub workflow reference required');
  assert(env.GITHUB_WORKFLOW_REF.includes('@'), 'Incomplete GitHub workflow reference');
  const workflow = env.GITHUB_WORKFLOW_REF.slice(prefix.length).split('@', 1)[0];
  assert(
    ['.github/workflows/ci.yml', '.github/workflows/v1-testing.yml'].includes(workflow),
    'Untrusted evidence workflow',
  );
  return {
    sha: env.GITHUB_SHA,
    producer: {
      kind: 'github-actions',
      repository: env.GITHUB_REPOSITORY,
      workflow,
      runId: Number(env.GITHUB_RUN_ID),
    },
  };
}

export function eligibleSuite(domains, suiteId) {
  const domain = domains.find((entry) => entry.suites.some((suite) => suite.id === suiteId));
  const suite = domain?.suites.find((entry) => entry.id === suiteId);
  assert(suite, 'Unknown V1 suite');
  assert.equal(suite.execution, 'automated', 'Operator suites cannot be produced by CI');
  assert(
    suite.cases.length === 1 && suite.cases[0] === `${suite.id}.complete-run`,
    'Multi-case business suites require case-level reports',
  );
  assert(/^[a-z0-9-]+:[a-z0-9:-]+$/u.test(suite.target), 'Suite requires an Nx target');
  return { domain: domain.domain, suite };
}

export function produceSuiteReceipt({ domains, suiteId, context, execute, clock, tools }) {
  const selected = eligibleSuite(domains, suiteId);
  assert(
    tools &&
      Object.keys(tools).length > 0 &&
      Object.values(tools).every((value) => typeof value === 'string' && value.trim().length > 0),
    'Tool versions required',
  );
  const command = ['pnpm', 'exec', 'nx', 'run', selected.suite.target];
  const startedAt = canonicalDate(clock());
  const execution = execute(command);
  const completedAt = canonicalDate(clock());
  assert(Date.parse(completedAt) >= Date.parse(startedAt), 'Receipt timestamps are reversed');
  const passed = execution.status === 0 && execution.signal == null && !execution.error;
  const result = {
    receiptVersion: 1,
    suite: selected.suite.id,
    domain: selected.domain,
    sha: context.sha,
    environment: 'isolated',
    status: passed ? 'passed' : 'failed',
    startedAt,
    completedAt,
    attempt: 1,
    tools,
    target: selected.suite.target,
    commandSha256: sha256(JSON.stringify(command)),
    producer: context.producer,
    cases: selected.suite.cases.map((id) => ({ id, status: passed ? 'passed' : 'failed' })),
  };
  return { result, passed };
}

function toolVersions() {
  const version = (command, args) =>
    execFileSync(command, args, { encoding: 'utf8', timeout: 10000 }).trim();
  return {
    node: process.version,
    pnpm: version('pnpm', ['--version']),
    nx: version('pnpm', ['exec', 'nx', '--version']),
  };
}

function outputPath(cwd, suiteId) {
  assert(/^[a-z0-9-]+\.[a-z0-9-]+$/u.test(suiteId), 'Invalid suite ID');
  return path.join(cwd, '.temp/v1-receipts', `${suiteId}.json`);
}

function main() {
  const suiteId = process.argv[2];
  assert(suiteId && process.argv.length === 3, 'usage: v1-suite-receipt.mjs <suite-id>');
  const cwd = process.cwd();
  const { domains } = loadV1(cwd);
  const { result, passed } = produceSuiteReceipt({
    domains,
    suiteId,
    context: ciContext(process.env),
    execute: ([command, ...args]) =>
      spawnSync(command, args, { cwd, stdio: 'inherit', shell: false }),
    clock: () => new Date(),
    tools: toolVersions(),
  });
  const output = outputPath(cwd, suiteId);
  mkdirSync(path.dirname(output), { recursive: true });
  const temporary = `${output}.${process.pid}.tmp`;
  writeFileSync(temporary, `${JSON.stringify(result, null, 2)}\n`, { mode: 0o600, flag: 'wx' });
  renameSync(temporary, output);
  console.log(`${result.status}: ${suiteId} receipt written to ${path.relative(cwd, output)}`);
  if (!passed) process.exitCode = 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    console.error(`V1 receipt refused: ${error.message}`);
    process.exitCode = 1;
  }
}
