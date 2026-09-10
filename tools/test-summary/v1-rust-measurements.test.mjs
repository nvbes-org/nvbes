import assert from 'node:assert/strict';
import { test } from 'node:test';
import { rustCoverage, rustMutation, verifyRustMeasurements } from './v1-rust-measurements.mjs';

const file = 'libs/rust/example/src/lib.rs';
const unit = {
  name: 'nvbes-example',
  language: 'rust',
  sourceRoot: 'libs/rust/example',
  sources: [file],
};
const phases = (build = 'Success', testStatus = 'Success') => [
  { phase: 'Build', process_status: build },
  ...(testStatus === undefined ? [] : [{ phase: 'Test', process_status: testStatus }]),
];
function fixture() {
  const coverage = {
    type: 'llvm.coverage.json.export',
    version: '3.1.0',
    data: [
      {
        files: [
          {
            filename: `/home/runner/work/nvbes/nvbes/${file}`,
            summary: {
              lines: { count: 10000, covered: 8999, percent: 100 },
              branches: { count: 3, covered: 2, percent: 100 },
            },
          },
        ],
      },
    ],
  };
  const mutation = {
    cargo_mutants_version: '27.1.0',
    start_time: '2026-09-10T10:00:00Z',
    end_time: '2026-09-10T11:00:00Z',
    total_mutants: 2,
    outcomes: [
      { scenario: 'Baseline', summary: 'Success', phase_results: phases() },
      ...['CaughtMutant', 'Timeout'].map((summary, i) => ({
        scenario: {
          Mutant: {
            package: unit.name,
            file,
            replacement: String(i),
            span: { start: { line: 1, column: 1 }, end: { line: 1, column: 2 } },
          },
        },
        summary,
        phase_results: phases('Success', i === 0 ? { Failure: 101 } : 'Timeout'),
      })),
    ],
  };
  return { coverage, mutation };
}

test('recomputes Rust counters without rounding or substituting region coverage', () => {
  const { coverage, mutation } = fixture();
  assert.equal(rustCoverage(unit, coverage, 'lines'), 89.99);
  assert.equal(rustCoverage(unit, coverage, 'branches'), 200 / 3);
  assert.equal(rustMutation(unit, mutation), 50);
});

for (const [name, change] of [
  [
    'missing branches',
    (r) => {
      delete r.data[0].files[0].summary.branches;
    },
  ],
  [
    'zero instrumentation',
    (r) => {
      r.data[0].files[0].summary.branches = { count: 0, covered: 0 };
    },
  ],
  [
    'fractional counter',
    (r) => {
      r.data[0].files[0].summary.branches.count = 3.5;
    },
  ],
  [
    'overcount',
    (r) => {
      r.data[0].files[0].summary.branches.covered = 4;
    },
  ],
  [
    'negative counter',
    (r) => {
      r.data[0].files[0].summary.branches.covered = -1;
    },
  ],
  [
    'duplicate file',
    (r) => {
      r.data[0].files.push(r.data[0].files[0]);
    },
  ],
  [
    'missing crate',
    (r) => {
      r.data[0].files = [];
    },
  ],
  [
    'unknown candidate file',
    (r) => {
      r.data[0].files[0].filename = 'libs/rust/example/src/missing.rs';
    },
  ],
  [
    'path traversal',
    (r) => {
      r.data[0].files[0].filename = `../${file}`;
    },
  ],
  [
    'unsupported LLVM',
    (r) => {
      r.version = '4.0.0';
    },
  ],
  [
    'unmerged exports',
    (r) => {
      r.data.push(r.data[0]);
    },
  ],
]) {
  test(`rejects LLVM ${String(name)}`, () => {
    const { coverage } = fixture();
    change(coverage);
    assert.throws(() => rustCoverage(unit, coverage, 'branches'));
  });
}

for (const [name, change] of [
  [
    'missing baseline',
    (r) => {
      r.outcomes.shift();
    },
  ],
  [
    'failed baseline',
    (r) => {
      r.outcomes[0].summary = 'Failure';
    },
  ],
  [
    'baseline without tests',
    (r) => {
      r.outcomes[0].phase_results.pop();
    },
  ],
  [
    'unknown status',
    (r) => {
      r.outcomes[1].summary = 'Pending';
    },
  ],
  [
    'duplicate mutant',
    (r) => {
      r.outcomes.push(r.outcomes[1]);
      r.total_mutants++;
    },
  ],
  [
    'missing outcome',
    (r) => {
      r.total_mutants++;
    },
  ],
  [
    'no viable mutant',
    (r) => {
      r.outcomes.splice(1);
      r.total_mutants = 0;
    },
  ],
  [
    'wrong crate source',
    (r) => {
      r.outcomes[1].scenario.Mutant.file = 'libs/rust/other/src/lib.rs';
    },
  ],
  [
    'unknown scenario',
    (r) => {
      r.outcomes[1].scenario = 'Unknown';
    },
  ],
  [
    'missing end time',
    (r) => {
      delete r.end_time;
    },
  ],
  [
    'reversed dates',
    (r) => {
      r.end_time = '2020-01-01T00:00:00Z';
    },
  ],
  [
    'unsupported version',
    (r) => {
      r.cargo_mutants_version = '28.0.0';
    },
  ],
  [
    'invalid position',
    (r) => {
      r.outcomes[1].scenario.Mutant.span.start.line = 0;
    },
  ],
  [
    'reversed span',
    (r) => {
      r.outcomes[1].scenario.Mutant.span.end.column = 0;
    },
  ],
  [
    'missing replacement',
    (r) => {
      delete r.outcomes[1].scenario.Mutant.replacement;
    },
  ],
  [
    'fake caught status',
    (r) => {
      r.outcomes[1].phase_results = phases();
    },
  ],
  [
    'timeout counted as caught',
    (r) => {
      r.outcomes[2].summary = 'CaughtMutant';
    },
  ],
  [
    'no execution phases',
    (r) => {
      delete r.outcomes[1].phase_results;
    },
  ],
  [
    'duplicate phase',
    (r) => {
      r.outcomes[1].phase_results.push(r.outcomes[1].phase_results[0]);
    },
  ],
]) {
  test(`rejects mutation ${String(name)}`, () => {
    const { mutation } = fixture();
    change(mutation);
    assert.throws(() => rustMutation(unit, mutation));
  });
}

test('excludes only unviable builds and keeps survived mutants in denominator', () => {
  const { mutation } = fixture();
  mutation.outcomes[2].summary = 'Unviable';
  mutation.outcomes[2].phase_results = [{ phase: 'Build', process_status: { Failure: 101 } }];
  assert.equal(rustMutation(unit, mutation), 100);
  mutation.outcomes[2].summary = 'MissedMutant';
  mutation.outcomes[2].phase_results = phases();
  assert.equal(rustMutation(unit, mutation), 50);
});

test('binds all three Rust reports to authenticated artifacts and exact declared scores', () => {
  const { coverage, mutation } = fixture();
  const artifacts = new Map([
    ['cov.json', Buffer.from(JSON.stringify(coverage))],
    ['mut.json', Buffer.from(JSON.stringify(mutation))],
  ]);
  const measurement = {
    unit: unit.name,
    lines: 89.99,
    branches: 200 / 3,
    mutation: 50,
    artifacts: [...artifacts.keys()],
    reports: { 'llvm-lines': 'cov.json', 'llvm-branches': 'cov.json', 'cargo-mutants': 'mut.json' },
  };
  const bundle = { measurements: [measurement] };
  assert.doesNotThrow(() => verifyRustMeasurements(bundle, [unit], artifacts));
  for (const metric of ['lines', 'branches', 'mutation']) {
    const original = measurement[metric];
    measurement[metric] = 100;
    assert.throws(() => verifyRustMeasurements(bundle, [unit], artifacts), /differs/u);
    measurement[metric] = original;
  }
  assert.throws(() => verifyRustMeasurements({ measurements: [] }, [unit], artifacts));
  assert.throws(() =>
    verifyRustMeasurements({ measurements: [measurement, measurement] }, [unit], artifacts),
  );
  measurement.artifacts.pop();
  assert.throws(() => verifyRustMeasurements(bundle, [unit], artifacts), /verified/u);
  measurement.artifacts = [...artifacts.keys()];
  artifacts.delete('mut.json');
  assert.throws(() => verifyRustMeasurements(bundle, [unit], artifacts), /verified/u);
});
