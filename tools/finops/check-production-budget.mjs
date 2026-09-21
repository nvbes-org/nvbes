import path from 'node:path';
import { loadBudgetContract } from './production-budget.mjs';

const cwd = process.cwd();
const rawPath = process.argv[2] ?? 'infrastructure/finops/production-budget.json';
const contractPath = path.resolve(cwd, rawPath);
const rel = path.relative(cwd, contractPath);
if (rel.startsWith('..') || path.isAbsolute(rel)) {
  console.error('Invalid path: contract must be within the workspace');
  process.exit(1);
}

try {
  const contract = await loadBudgetContract(contractPath);
  console.log(
    `FinOps production budget passed: target=${contract.monthly.targetCents} cents, hard=${contract.monthly.hardLimitCents} cents.`,
  );
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
