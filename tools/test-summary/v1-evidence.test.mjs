import assert from 'node:assert/strict';
import { test } from 'node:test';
import { evaluateV1 } from './v1-evidence.mjs';
import { loadV1 } from './v1-release.mjs';
import { cargoClosure, typescriptThresholds } from './v1-catalogue.mjs';
import { validateManifest } from './v1-manifest.mjs';

export function validCandidate() {
  const { root, domains, manifestDigest } = loadV1(process.cwd());
  // Fixtures model a future fully verified candidate, not current repository evidence.
  for (const domain of domains) domain.gaps = [];
  const expectedSha = 'a'.repeat(40);
  const time = '2026-09-10T12:00:00.000Z';
  const results = domains.flatMap((domain) =>
    domain.suites.map((suite) => ({
      suite: suite.id,
      domain: domain.domain,
      sha: expectedSha,
      environment: 'isolated',
      status: 'passed',
      startedAt: time,
      completedAt: time,
      attempt: 1,
      tools: { harness: '1.0.0' },
      artifacts: ['report.json'],
      producer: { kind: suite.execution === 'human' ? 'operator' : 'github-actions' },
      cases: suite.cases.map((id) => ({ id, status: 'passed' })),
    })),
  );
  const units = [{ name: 'nvbes-core', thresholds: {} }];
  return {
    root,
    domains,
    manifestDigest,
    units,
    expectedSha,
    verified: true,
    now: new Date(time),
    bundle: {
      schemaVersion: 1,
      sha: expectedSha,
      manifestDigest,
      results,
      artifacts: [{ path: 'report.json' }],
      measurements: [
        {
          unit: 'nvbes-core',
          sha: expectedSha,
          lines: 90,
          branches: 90,
          mutation: 90,
          completedAt: time,
          artifacts: ['report.json'],
        },
      ],
      operations: { rpoHours: 24, rtoHours: 8, targetMonthlyEurTtc: 20, maximumMonthlyEurTtc: 30 },
    },
  };
}

test('preserves stronger TypeScript CLI coverage thresholds', () => {
  assert.deepEqual(
    typescriptThresholds({
      scripts: {
        coverage: 'vp test --coverage.thresholds.lines=95 --coverage.thresholds.branches=90',
      },
    }),
    { lines: 95, branches: 90 },
  );
});

test('a complete verified candidate passes at the exact boundaries', () => {
  assert.deepEqual(evaluateV1(validCandidate()).failures, []);
});

for (const [name, mutate, pattern] of [
  [
    'unverified packet',
    (input) => {
      input.verified = false;
    },
    /not verified/u,
  ],
  [
    'wrong SHA',
    (input) => {
      input.bundle.sha = 'b'.repeat(40);
    },
    /another SHA/u,
  ],
  [
    'wrong manifest',
    (input) => {
      input.bundle.manifestDigest = 'b'.repeat(64);
    },
    /another manifest/u,
  ],
  [
    'missing bundle',
    (input) => {
      input.bundle = undefined;
    },
    /Missing/u,
  ],
  [
    'missing suite',
    (input) => {
      input.bundle.results.pop();
    },
    /not-run/u,
  ],
  [
    'duplicate suite',
    (input) => {
      input.bundle.results.push(input.bundle.results[0]);
    },
    /Duplicate/u,
  ],
  [
    'wrong producer',
    (input) => {
      input.bundle.results[0].producer.kind = 'operator';
    },
    /producer/u,
  ],
  [
    'unknown status',
    (input) => {
      input.bundle.results[0].status = 'ok';
    },
    /: ok/u,
  ],
  [
    'skipped suite',
    (input) => {
      input.bundle.results[0].status = 'not-applicable';
    },
    /not-applicable/u,
  ],
  [
    'rerun',
    (input) => {
      input.bundle.results[0].attempt = 2;
    },
    /retry/u,
  ],
  [
    'missing case',
    (input) => {
      input.bundle.results[0].cases.pop();
    },
    /incomplete test cases/u,
  ],
  [
    'missing tools',
    (input) => {
      input.bundle.results[0].tools = {};
    },
    /tool versions/u,
  ],
  [
    'old proof',
    (input) => {
      input.now = new Date('2026-09-12T12:00:00.000Z');
    },
    /stale/u,
  ],
  [
    'future proof',
    (input) => {
      input.bundle.results[0].completedAt = '2027-01-01T00:00:00.000Z';
    },
    /timestamps/u,
  ],
  [
    'invalid date',
    (input) => {
      input.bundle.results[0].startedAt = '2026-02-30T12:00:00.000Z';
    },
    /timestamps/u,
  ],
  [
    'missing measurement',
    (input) => {
      input.bundle.measurements = [];
    },
    /missing measurement/u,
  ],
  [
    'unverified measurement file',
    (input) => {
      input.bundle.measurements[0].artifacts = ['missing'];
    },
    /measurement artifacts/u,
  ],
  [
    'NaN score',
    (input) => {
      input.bundle.measurements[0].lines = Number.NaN;
    },
    /minimum/u,
  ],
  [
    'RPO failure',
    (input) => {
      input.bundle.operations.rpoHours = 24.01;
    },
    /restoration/u,
  ],
  [
    'RTO failure',
    (input) => {
      input.bundle.operations.rtoHours = 8.01;
    },
    /restoration/u,
  ],
  [
    'budget failure',
    (input) => {
      input.bundle.operations.maximumMonthlyEurTtc = 30.01;
    },
    /FinOps/u,
  ],
  [
    'empty measurement scope',
    (input) => {
      input.units = [];
    },
    /production measurement/u,
  ],
]) {
  test(`refuses ${String(name)}`, () => {
    const input = validCandidate();
    mutate(input);
    const result = evaluateV1(input);
    assert.equal(result.verdict, 'NO-GO');
    assert.match(result.failures.join('\n'), pattern);
  });
}

for (const metric of ['lines', 'branches', 'mutation']) {
  test(`refuses ${metric} at 89.99% without rounding up`, () => {
    const input = validCandidate();
    input.bundle.measurements[0][metric] = 89.99;
    assert.equal(evaluateV1(input).verdict, 'NO-GO');
  });
}

test('preserves a pre-existing threshold above 90%', () => {
  const input = validCandidate();
  input.units[0].thresholds.branches = 95;
  assert.equal(evaluateV1(input).verdict, 'NO-GO');
});

test('the actual V1 gap registry cannot yield GO with synthetic passing results', () => {
  const input = validCandidate();
  input.domains = loadV1(process.cwd()).domains;
  assert.match(evaluateV1(input).failures.join('\n'), /open gap/u);
});

test('a missing domain and duplicate case invalidate the manifest', () => {
  const input = validCandidate();
  assert.throws(() => validateManifest(input.root, input.domains.slice(1)), /Six/u);
  input.domains[1].suites[0].cases[0] = input.domains[0].suites[0].cases[0];
  assert.throws(() => validateManifest(input.root, input.domains), /Duplicate test case/u);
});

test('production closure follows local dependencies, including cycles, but excludes dev dependencies', () => {
  const packages = [
    {
      name: 'app',
      dependencies: [
        { name: 'core', path: '/core', kind: null },
        { name: 'tests', path: '/tests', kind: 'dev' },
      ],
    },
    { name: 'core', dependencies: [{ name: 'app', path: '/app', kind: null }] },
  ];
  assert.deepEqual(cargoClosure(packages, ['app']), ['app', 'core']);
  assert.throws(() => cargoClosure(packages, ['missing']), /Missing Cargo/u);
});
