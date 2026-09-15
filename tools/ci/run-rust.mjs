import { execFileSync, spawnSync } from 'node:child_process';
import { appendFileSync, readFileSync } from 'node:fs';
import { selectAffectedCargoPackages } from './cargo-affected.core.mjs';
import { checkOomExit, startMemoryWatchdog, stopMemoryWatchdog } from './memory-guard.mjs';

const plan = JSON.parse(process.env.NVBES_CI_PLAN);
const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'));
if (plan.version !== 1 || !plan.candidate.rust || !['workspace', 'scoped'].includes(plan.rustMode))
  throw new Error('Invalid Rust execution plan');
const metadata = JSON.parse(
  execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--no-deps'], {
    encoding: 'utf8',
  }),
);
const members = metadata.packages
  .filter((p) => metadata.workspace_members.includes(p.id))
  .map((p) => p.name)
  .sort();
const packages = plan.fallbackFull
  ? members
  : selectAffectedCargoPackages(metadata, plan.paths, process.cwd());
if (!packages.length || packages.some((name) => !members.includes(name)))
  throw new Error('Invalid Rust candidate scope');
if (spawnSync('cargo', ['fmt', '--all', '--check'], { stdio: 'inherit' }).status !== 0)
  throw new Error('Rust formatting failed');
if (
  spawnSync(
    'cargo',
    ['clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings'],
    { stdio: 'inherit' },
  ).status !== 0
)
  throw new Error('Rust linting failed');
if (
  spawnSync('pnpm', ['exec', 'nx', 'run', 'rust-workspace:micro-test'], { stdio: 'inherit' })
    .status !== 0
)
  throw new Error('Isolated micro-tests failed');
startMemoryWatchdog({ intervalMs: 2000, thresholdMb: 500 });
const started = Date.now();
const scoped = spawnSync(
  'cargo',
  ['test', '--locked', ...packages.flatMap((name) => ['--package', name])],
  { stdio: 'inherit' },
);
checkOomExit(scoped.status, scoped.signal, 'cargo test scoped');
// Avoid an identical second execution for global changes. For smaller scopes,
// run the full baseline even when the candidate fails so divergence is visible.
const full =
  plan.rustMode === 'scoped' || packages.length === members.length
    ? scoped
    : spawnSync('cargo', ['test', '--workspace', '--locked'], { stdio: 'inherit' });
if (full !== scoped) {
  checkOomExit(full.status, full.signal, 'cargo test workspace');
}
const memoryStats = stopMemoryWatchdog();
const evidence = {
  version: 1,
  runId: process.env.GITHUB_RUN_ID,
  head: plan.head,
  sourceHead: event.pull_request?.head?.sha ?? plan.head,
  base: plan.base,
  event: process.env.GITHUB_EVENT_NAME,
  timestamp: new Date().toISOString(),
  pr: process.env.GITHUB_REF?.match(/^refs\/pull\/(\d+)\/merge$/u)?.[1] ?? null,
  packages,
  workspace: members,
  scopedSuccess: scoped.status === 0,
  fullSuccess: plan.rustMode === 'workspace' ? full.status === 0 : null,
  baselineExecuted: plan.rustMode === 'workspace',
  divergence: (scoped.status === 0) !== (full.status === 0),
  durationMs: Date.now() - started,
  minMemoryAvailableMb: Number.isFinite(memoryStats.minAvailableMbSeen)
    ? memoryStats.minAvailableMbSeen
    : null,
};
console.log(`CI_RUST_EVIDENCE ${JSON.stringify(evidence)}`);
appendFileSync(
  process.env.GITHUB_STEP_SUMMARY,
  `## Rust shadow comparison\n\n\`\`\`json\n${JSON.stringify(evidence, null, 2)}\n\`\`\`\n`,
);
if (full.status !== 0 || scoped.status !== 0)
  throw new Error('Rust candidate or workspace baseline failed');
