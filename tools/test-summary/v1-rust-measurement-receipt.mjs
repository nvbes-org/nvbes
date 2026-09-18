import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { confinedRead } from './v1-bundle-verification.mjs';
import { productionUnits } from './v1-catalogue.mjs';
import { loadV1 } from './v1-release.mjs';
import { rustCoverage, rustMutation } from './v1-rust-measurements.mjs';
import { ciContext } from './v1-suite-receipt.mjs';

const reports = {
  'llvm-lines': 'rust/llvm.json',
  'llvm-branches': 'rust/llvm.json',
  'cargo-mutants': 'rust/mutation.json',
};
const artifacts = ['rust/measurements.json', 'rust/llvm.json', 'rust/mutation.json'];

export function produceRustMeasurements({ units, context, llvmBytes, mutationBytes, completedAt }) {
  const llvm = JSON.parse(llvmBytes.toString('utf8'));
  const mutation = JSON.parse(mutationBytes.toString('utf8'));
  return units.map((unit) => {
    const scores = {
      lines: rustCoverage(unit, llvm, 'lines'),
      branches: rustCoverage(unit, llvm, 'branches'),
      mutation: rustMutation(unit, mutation),
    };
    for (const [metric, score] of Object.entries(scores)) {
      const minimum = Math.max(90, unit.thresholds?.[metric] ?? 0);
      assert(score >= minimum, `${unit.name}: ${metric} ${score} (minimum ${minimum}%)`);
    }
    return {
      measurementVersion: 1,
      language: 'rust',
      unit: unit.name,
      sha: context.sha,
      completedAt: new Date(completedAt).toISOString(),
      attempt: 1,
      tools: context.tools,
      producer: context.producer,
      reports,
      artifacts,
      ...scores,
    };
  });
}

function main() {
  assert(
    process.argv.length === 4 &&
      process.argv[2].startsWith('--llvm=') &&
      process.argv[3].startsWith('--mutation='),
    'usage: --llvm=<path> --mutation=<path>',
  );
  const cwd = process.cwd();
  const { root, domains } = loadV1(cwd);
  const units = productionUnits(root, domains, cwd).filter((unit) => unit.language === 'rust');
  const llvmBytes = confinedRead(cwd, process.argv[2].slice('--llvm='.length));
  const mutationBytes = confinedRead(cwd, process.argv[3].slice('--mutation='.length));
  const context = ciContext(process.env);
  context.tools = {
    rust: process.env.NVBES_RUST_VERSION,
    llvmCov: process.env.NVBES_LLVM_COV_VERSION,
    cargoMutants: process.env.NVBES_CARGO_MUTANTS_VERSION,
  };
  assert(Object.values(context.tools).every((value) => typeof value === 'string' && value.trim()));
  const measurements = produceRustMeasurements({
    units,
    context,
    llvmBytes,
    mutationBytes,
    completedAt: new Date(),
  });
  const output = path.join(cwd, '.temp/v1-measurements/rust');
  mkdirSync(output, { recursive: true });
  for (const [name, bytes] of [
    ['llvm.json', llvmBytes],
    ['mutation.json', mutationBytes],
  ]) {
    writeFileSync(path.join(output, name), bytes, { mode: 0o600, flag: 'wx' });
  }
  writeFileSync(
    path.join(output, 'measurements.json'),
    `${JSON.stringify(measurements, null, 2)}\n`,
    {
      mode: 0o600,
      flag: 'wx',
    },
  );
  console.log(`V1 Rust measurements written for ${measurements.length} units`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    console.error(`V1 Rust measurement refused: ${error.message}`);
    process.exitCode = 1;
  }
}
