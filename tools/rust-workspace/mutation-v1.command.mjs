import assert from 'node:assert/strict';
import path from 'node:path';

export function mutationCommand(manifest, packages, output, env = process.env) {
  const jobs = env.NVBES_MUTATION_JOBS ?? '2';
  const timeout = env.NVBES_MUTATION_TIMEOUT ?? '600';
  assert(/^[1-9]\d*$/u.test(jobs) && Number(jobs) <= 4, 'Mutation concurrency must be 1..4');
  assert(/^[1-9]\d*$/u.test(timeout), 'Mutation timeout must be positive seconds');
  return [
    'mutants',
    '--manifest-path',
    path.resolve(manifest),
    ...packages.flatMap((name) => ['--package', name]),
    '--jobs',
    jobs,
    '--timeout',
    timeout,
    '--cargo-arg=--locked',
    '--no-times',
    '-o',
    output,
  ];
}
