import { readFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { deriveReleaseBlockingCategories } from './check-account-release-readiness.mjs';
import { validateAccountQualityWorkflow } from './workflow-contract.mjs';

export const validIso29119Techniques = [
  'equivalence-partitioning',
  'boundary-value-analysis',
  'state-transition',
  'cause-effect',
  'decision-table',
  'use-case',
  'error-guessing',
  'risk-based',
  'exploratory-testing',
  'statement-coverage',
  'branch-coverage',
  'condition-coverage',
  'path-coverage',
  'basis-path-coverage',
  'mutation-testing',
];

export const requiredCategories = [
  'acceptance-alpha',
  'acceptance-beta',
  'access-control',
  'accessibility',
  'api-esb',
  'authentication-authorization-rbac',
  'beta',
  'black-box',
  'compatibility',
  'compliance',
  'component-integration',
  'contract',
  'cookies',
  'cross-browser',
  'cross-platform',
  'data-migration',
  'drp-failover',
  'end-to-end',
  'end-to-end-integration',
  'endurance',
  'exploratory',
  'fuzz',
  'global-regression',
  'grey-box',
  'interface',
  'load',
  'localization',
  'maintainability',
  'monkey',
  'penetration',
  'performance',
  'recovery',
  'reliability',
  'sanity',
  'scalability',
  'security',
  'smoke',
  'spike',
  'stress',
  'system-integration',
  'ui',
  'unit',
  'usability',
  'user-acceptance',
  'volume',
  'white-box',
];

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDirectory, '../..');

if (path.resolve(process.argv[1] || '') === fileURLToPath(import.meta.url)) {
  const manifest = JSON.parse(
    await readFile(path.join(workspaceRoot, 'docs/testing/account-test-manifest.json'), 'utf8'),
  );
  const qualityProject = JSON.parse(
    await readFile(path.join(scriptDirectory, 'project.json'), 'utf8'),
  );
  const workflowSource = await readFile(
    path.join(workspaceRoot, '.github/workflows/account-quality.yml'),
    'utf8',
  );
  validateTestManifest(manifest, qualityProject, workflowSource);
  process.stdout.write(
    `Account test manifest valid: ${manifest.coverage.length} honest category contracts.\n`,
  );
}

export function validateTestManifest(manifest, qualityProject, workflowSource) {
  assert(manifest.schemaVersion === 4, 'schemaVersion must be 4');
  assert(isRecord(manifest.scope), 'scope must be an object');
  assert(isRecord(manifest.automation), 'automation must be an object');
  assert(isRecord(manifest.lanes), 'lanes must be an object');
  assert(Array.isArray(manifest.coverage), 'coverage must be an array');
  assert(Array.isArray(manifest.knownGaps), 'knownGaps must be an array');
  assert(isRecord(manifest.releaseReadiness), 'releaseReadiness must be an object');
  assert(isRecord(manifest.policy), 'policy must be an object');
  assert(
    qualityProject.targets?.test?.inputs?.includes('{workspaceRoot}/tools/load-tests/account/**/*'),
    'account-quality:test inputs must cover the complete Account load suite',
  );

  for (const application of ['account-service', 'account-worker', 'account-web']) {
    assert(manifest.scope.applications?.includes(application), `scope is missing ${application}`);
  }

  const categoryContracts = new Map();
  for (const contract of manifest.coverage) {
    validateCoverageContract(contract, manifest.lanes, qualityProject);
    assert(
      !categoryContracts.has(contract.category),
      `${contract.category} has more than one coverage contract`,
    );
    categoryContracts.set(contract.category, contract);
  }

  const missing = requiredCategories.filter((category) => !categoryContracts.has(category));
  const unexpected = [...categoryContracts.keys()].filter(
    (category) => !requiredCategories.includes(category),
  );
  assert(missing.length === 0, `missing categories: ${missing.join(', ')}`);
  assert(unexpected.length === 0, `unexpected categories: ${unexpected.join(', ')}`);
  validateKnownGaps(manifest.knownGaps, categoryContracts);
  const derivedBlockers = deriveReleaseBlockingCategories(manifest);
  const declaredBlockers = [...(manifest.releaseReadiness.blockingCategories ?? [])].sort();
  assert(
    JSON.stringify(declaredBlockers) === JSON.stringify(derivedBlockers),
    'releaseReadiness.blockingCategories must exactly match gaps, limitations and blocking known gaps',
  );
  assert(
    manifest.releaseReadiness.status === (derivedBlockers.length > 0 ? 'blocked' : 'ready'),
    'releaseReadiness.status does not match blockingCategories',
  );

  assertGap(categoryContracts, 'fuzz');
  assertGap(categoryContracts, 'cross-platform');
  assertGap(categoryContracts, 'scalability');
  assert(
    categoryContracts.get('scalability')?.blocker.includes('control-plane'),
    'scalability gap must name the missing control-plane verifier',
  );
  assert(
    categoryContracts.get('drp-failover')?.execution === 'human',
    'DRP/failover must not be advertised as automated',
  );

  validateScheduledLanes(categoryContracts);
  validateAccountQualityWorkflow(workflowSource, manifest.automation);
  assert(manifest.policy.silentSkipsAllowed === false, 'silent skips must be forbidden');
  assert(manifest.policy.retryCanMakeGateGreen === false, 'retry-green must be forbidden');
  assert(
    manifest.policy.productionLoadTestingAllowed === false,
    'production load tests must be forbidden',
  );
  assert(
    manifest.policy.secretsInArtifactsAllowed === false,
    'secrets in artifacts must be forbidden',
  );
}

function validateCoverageContract(contract, lanes, qualityProject) {
  assert(isRecord(contract), 'each coverage contract must be an object');
  assert(requiredCategories.includes(contract.category), `unknown category ${contract.category}`);
  assert(isRecord(lanes[contract.lane]), `${contract.category} references an unknown lane`);
  assert(
    ['automated', 'gap', 'human', 'manual-command'].includes(contract.execution),
    `${contract.category} has an invalid execution mode`,
  );
  assert(
    Array.isArray(contract.iso29119Techniques) && contract.iso29119Techniques.length > 0,
    `${contract.category}.iso29119Techniques must be a non-empty array`,
  );
  for (const technique of contract.iso29119Techniques) {
    assert(
      validIso29119Techniques.includes(technique),
      `${contract.category} references unknown ISO 29119-4 technique: ${technique}`,
    );
  }
  const laneMode = lanes[contract.lane].mode;
  if ('limitation' in contract) {
    assert(nonEmpty(contract.limitation), `${contract.category}.limitation must be non-empty`);
  }
  assert(
    laneMode === contract.execution ||
      (laneMode === 'human' && contract.execution === 'manual-command'),
    `${contract.category} execution does not match lane ${contract.lane}`,
  );

  if (contract.execution === 'automated' || contract.execution === 'manual-command') {
    assert(nonEmpty(contract.command), `${contract.category}.command is required`);
    assert(nonEmpty(contract.artifact), `${contract.category}.artifact is required`);
    assert(!contract.runbook, `${contract.category} cannot also declare a runbook`);
    const targetMatch = contract.command.match(/\bnx run account-quality:([\w-]+)/u);
    if (targetMatch) {
      assert(
        isRecord(qualityProject.targets?.[targetMatch[1]]),
        `${contract.category} references missing target ${targetMatch[1]}`,
      );
    }
  } else if (contract.execution === 'human') {
    assert(nonEmpty(contract.runbook), `${contract.category}.runbook is required`);
    assert(nonEmpty(contract.artifact), `${contract.category}.artifact is required`);
    assert(!contract.command, `${contract.category} human validation cannot claim a command`);
  } else {
    assert(nonEmpty(contract.blocker), `${contract.category}.blocker is required`);
    assert(
      !contract.command && !contract.artifact,
      `${contract.category} gap cannot claim evidence`,
    );
  }
}

function validateKnownGaps(knownGaps, categoryContracts) {
  for (const [index, knownGap] of knownGaps.entries()) {
    assert(isRecord(knownGap), `knownGaps[${index}] must be an object`);
    assert(nonEmpty(knownGap.description), `knownGaps[${index}].description is required`);
    assert(
      Array.isArray(knownGap.categories) && knownGap.categories.length > 0,
      `knownGaps[${index}].categories is required`,
    );
    for (const category of knownGap.categories) {
      assert(
        categoryContracts.has(category),
        `knownGaps[${index}] references unknown category ${category}`,
      );
    }
    assert(
      typeof knownGap.releaseBlocking === 'boolean',
      `knownGaps[${index}].releaseBlocking must be boolean`,
    );
    if (!knownGap.releaseBlocking) {
      assert(
        nonEmpty(knownGap.control),
        `knownGaps[${index}] needs a control when it is not release-blocking`,
      );
    }
  }
}

function validateScheduledLanes(contracts) {
  for (const category of ['accessibility', 'compatibility', 'cookies', 'cross-browser', 'ui']) {
    assert(
      contracts.get(category).lane === 'scheduledDaily',
      `${category} must remain in scheduledDaily`,
    );
  }
  for (const category of ['spike', 'stress', 'volume']) {
    assert(
      contracts.get(category).lane === 'scheduledWeekly',
      `${category} must remain in scheduledWeekly`,
    );
  }
}

function assertGap(contracts, category) {
  assert(contracts.get(category)?.execution === 'gap', `${category} must remain an explicit gap`);
}

function nonEmpty(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
