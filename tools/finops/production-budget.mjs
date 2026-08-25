import { readFile } from 'node:fs/promises';

export const BudgetStage = Object.freeze({
  normal: 'normal',
  disableNonEssential: 'disable_non_essential',
  freezeCostCreation: 'freeze_cost_creation',
  essentialOnly: 'essential_only',
});

const REQUIRED_CATEGORIES = Object.freeze([
  'domain_dns',
  'compute',
  'postgres',
  'email',
  'storage_registry',
  'observability',
  'safety_margin',
]);

const REQUIRED_STAGES = Object.freeze([
  { name: BudgetStage.normal, maximumStartCents: 0 },
  { name: BudgetStage.disableNonEssential, maximumStartCents: 2500 },
  { name: BudgetStage.freezeCostCreation, maximumStartCents: 2800 },
  { name: BudgetStage.essentialOnly, maximumStartCents: 3000 },
]);

const isRecord = (value) => value !== null && typeof value === 'object' && !Array.isArray(value);
const isCents = (value) => Number.isInteger(value) && value >= 0;

export function validateBudgetContract(contract) {
  const errors = [];
  const monthly = isRecord(contract) && isRecord(contract.monthly) ? contract.monthly : {};
  const targetCents = monthly.targetCents;
  const hardLimitCents = monthly.hardLimitCents;

  if (!isRecord(contract)) {
    throw new Error('Invalid production FinOps budget: contract must be an object');
  }
  if (contract.version !== 1) errors.push('version must be 1');
  if (contract.currency !== 'EUR') errors.push('currency must be EUR');
  if (contract.taxesIncluded !== true) errors.push('taxesIncluded must be true');
  if (!isCents(targetCents)) errors.push('monthly target must be a non-negative integer in cents');
  else if (targetCents > 2000) errors.push('monthly target must not exceed 2000 cents');
  if (!isCents(hardLimitCents)) errors.push('monthly hard limit must be a non-negative integer in cents');
  else if (hardLimitCents > 3000) errors.push('monthly hard limit must not exceed 3000 cents');
  if (isCents(targetCents) && isCents(hardLimitCents)) {
    if (targetCents > hardLimitCents) errors.push('monthly target must not exceed monthly hard limit');
    if (hardLimitCents - targetCents < 1000) errors.push('reserve must be at least 1000 cents');
  }

  if (!Array.isArray(contract.alerts) || contract.alerts.length === 0) {
    errors.push('alerts must be a non-empty array');
  } else {
    if (contract.alerts.some((alert) => !isCents(alert))) errors.push('alerts must contain non-negative integer cents');
    if (contract.alerts.some((alert, index) => index > 0 && alert <= contract.alerts[index - 1])) {
      errors.push('alerts must be strictly increasing');
    }
    if (isCents(hardLimitCents) && contract.alerts.at(-1) !== hardLimitCents) {
      errors.push('final alert must equal monthly hard limit');
    }
  }

  if (!isRecord(contract.categories)) {
    errors.push('categories must be an object');
  } else {
    const keys = Object.keys(contract.categories);
    if (keys.length !== REQUIRED_CATEGORIES.length || REQUIRED_CATEGORIES.some((category) => !keys.includes(category))) {
      errors.push('categories must contain exactly the required keys');
    }
    let categoryTargetSum = 0;
    for (const category of REQUIRED_CATEGORIES) {
      const entry = contract.categories[category];
      if (!isRecord(entry) || !isCents(entry.targetCents) || !isCents(entry.limitCents)) {
        errors.push(`${category} target and limit must be non-negative integer cents`);
        continue;
      }
      categoryTargetSum += entry.targetCents;
      if (entry.targetCents > entry.limitCents) errors.push(`${category} target must not exceed limit`);
    }
    if (isCents(targetCents) && categoryTargetSum !== targetCents) {
      errors.push('category target sum must equal monthly target');
    }
  }

  if (!Array.isArray(contract.stages) || contract.stages.length !== REQUIRED_STAGES.length) {
    errors.push('stages must contain exactly the four required stages');
  } else {
    let previousStart = -1;
    for (const [index, required] of REQUIRED_STAGES.entries()) {
      const stage = contract.stages[index];
      if (!isRecord(stage) || stage.name !== required.name) errors.push(`stage ${index + 1} must be ${required.name}`);
      if (!isRecord(stage) || !isCents(stage.startsAtCents)) {
        errors.push(`${required.name} stage start must be a non-negative integer in cents`);
        continue;
      }
      if (stage.startsAtCents > required.maximumStartCents) {
        errors.push(`${required.name} stage must start no later than ${required.maximumStartCents} cents`);
      }
      if (stage.startsAtCents <= previousStart) errors.push('stage starts must be strictly increasing');
      previousStart = stage.startsAtCents;
    }
    if (isRecord(contract.stages[0]) && contract.stages[0].startsAtCents !== 0) {
      errors.push('normal stage must start at 0 cents');
    }
  }

  if (errors.length > 0) throw new Error(`Invalid production FinOps budget: ${errors.join('; ')}`);
  return contract;
}

export function stageForSpend(contract, spendCents) {
  validateBudgetContract(contract);
  if (!isCents(spendCents)) throw new TypeError('spend must be a non-negative integer in cents');
  let activeStage = contract.stages[0].name;
  for (const stage of contract.stages) {
    if (spendCents >= stage.startsAtCents) activeStage = stage.name;
  }
  return activeStage;
}

export async function loadBudgetContract(file) {
  const content = await readFile(file, 'utf8');
  return validateBudgetContract(JSON.parse(content));
}
