import path from 'node:path';
import { loadBudgetContract } from './production-budget.mjs';

const contractPath = path.resolve(
  process.argv[2] ?? 'infrastructure/finops/production-budget.json',
);

try {
  const contract = await loadBudgetContract(contractPath);
  console.log(
    `FinOps production budget passed: target=${contract.monthly.targetCents} cents, hard=${contract.monthly.hardLimitCents} cents.`,
  );
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
