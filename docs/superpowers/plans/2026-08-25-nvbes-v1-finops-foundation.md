# nvbes V1 FinOps Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the approved 20 EUR target and 30 EUR hard monthly production budget enforceable in the repository before another nvbes V1 runtime is deployed.

**Architecture:** A versioned JSON contract is the infrastructure source of truth for the tax-inclusive monthly budget, category envelopes and degradation stages. Node checks validate that contract and reject Scaleway containers or Serverless SQL databases whose literal scale bounds can exceed the V1 policy; a pure Rust state machine exposes the same degradation semantics to future Platform Operations and product runtimes without coupling the domain layer to Scaleway.

**Tech Stack:** Node.js 24 test runner and ES modules, JSON, Terraform HCL static policy checks, Rust 2024, Serde, thiserror, pnpm, Nx.

---

## Scope boundary and program decomposition

This plan implements the first independently releasable slice of
`2026-08-25-nvbes-v1-finops-platform-operations-design.md`. The active
workspace currently contains only `email-worker` and `trust-risk-service` as
runtime applications. Creating Identity, Account, Billing and Platform
Operations in this plan would combine independent data models, APIs and release
gates into one unreviewable change.

The V1 program is therefore split as follows:

1. FinOps foundation — this plan.
2. Email V1 acceptance audit against the existing global email implementation.
3. Identity and Account runtime design and implementation plan.
4. Billing runtime design and implementation plan.
5. Trust/Risk V1 acceptance audit against its existing V0 implementation plan.
6. Platform Operations cases, operator commands and live spend enforcement.
7. Integrated invited-user rollout and restoration rehearsal.

Each later slice must produce working software and pass the FinOps gates added
here. This plan intentionally does not scaffold an absent runtime.

## File map

### Production budget contract

- `infrastructure/finops/production-budget.json`: approved tax-inclusive budget, category targets, hard limit, alerts and degradation thresholds.
- `infrastructure/finops/README.md`: operating procedure, ownership and rules for changing the contract.
- `tools/finops/production-budget.mjs`: pure parsing, validation and spend-stage selection.
- `tools/finops/production-budget.test.mjs`: contract and boundary tests.
- `tools/finops/check-production-budget.mjs`: CI command that validates the repository contract.
- `package.json`: root `check:finops` target and root-check integration.

### Terraform cost ceilings

- `tools/finops/check-scale-to-zero.mjs`: validate both minimum and maximum literal scaling values.
- `tools/finops/check-scale-to-zero.test.mjs`: accepted and rejected Terraform fixtures.
- `infrastructure/environments/email-production/email-delivery.tf`: cap the Email runtime at one instance.
- `infrastructure/environments/trust-risk-production/runtime.tf`: cap the Trust/Risk runtime at one instance.
- `infrastructure/environments/trust-risk-production/database.tf`: cap the Trust/Risk database at one vCPU.

### Reusable runtime policy

- `libs/rust/platform/src/platform.finops.budget.rs`: validated thresholds and pure budget-stage selection.
- `libs/rust/platform/src/platform.finops.budget.tests.rs`: boundary and invalid-policy tests.
- `libs/rust/platform/src/lib.rs`: explicit FinOps module and public exports.

### Documentation

- `infrastructure/README.md`: link the production infrastructure overview to the executable FinOps contract.

## Task 1: Add the executable production budget contract

**Files:**

- Create: `tools/finops/production-budget.test.mjs`
- Create: `tools/finops/production-budget.mjs`
- Create: `infrastructure/finops/production-budget.json`

- [ ] **Step 1: Write the failing budget-contract tests**

Create `tools/finops/production-budget.test.mjs`:

```javascript
import assert from 'node:assert/strict';
import test from 'node:test';
import {
  BudgetStage,
  stageForSpend,
  validateBudgetContract,
} from './production-budget.mjs';

function validContract() {
  return {
    version: 1,
    currency: 'EUR',
    taxesIncluded: true,
    monthly: {
      targetCents: 2_000,
      hardLimitCents: 3_000,
      alertsCents: [1_500, 2_000, 2_500, 2_800, 3_000],
    },
    stages: [
      { name: BudgetStage.Normal, fromCents: 0 },
      { name: BudgetStage.DisableNonEssential, fromCents: 2_500 },
      { name: BudgetStage.FreezeCostCreation, fromCents: 2_800 },
      { name: BudgetStage.EssentialOnly, fromCents: 3_000 },
    ],
    categories: {
      domain_dns: { targetCents: 200, limitCents: 300 },
      compute: { targetCents: 300, limitCents: 600 },
      postgres: { targetCents: 400, limitCents: 800 },
      email: { targetCents: 100, limitCents: 200 },
      storage_registry: { targetCents: 200, limitCents: 400 },
      observability: { targetCents: 0, limitCents: 200 },
      safety_margin: { targetCents: 800, limitCents: 800 },
    },
  };
}

test('accepts the approved tax-inclusive V1 budget', () => {
  assert.doesNotThrow(() => validateBudgetContract(validContract()));
});

test('rejects a target or hard limit above the approved ceilings', () => {
  const targetTooHigh = validContract();
  targetTooHigh.monthly.targetCents = 2_001;
  assert.throws(
    () => validateBudgetContract(targetTooHigh),
    /monthly target must not exceed 2000 cents/,
  );

  const hardLimitTooHigh = validContract();
  hardLimitTooHigh.monthly.hardLimitCents = 3_001;
  assert.throws(
    () => validateBudgetContract(hardLimitTooHigh),
    /monthly hard limit must not exceed 3000 cents/,
  );
});

test('rejects missing reserve or category allocation drift', () => {
  const missingReserve = validContract();
  missingReserve.monthly.targetCents = 2_500;
  assert.throws(
    () => validateBudgetContract(missingReserve),
    /monthly reserve must be at least 1000 cents/,
  );

  const allocationDrift = validContract();
  allocationDrift.categories.compute.targetCents = 301;
  assert.throws(
    () => validateBudgetContract(allocationDrift),
    /category targets must equal the monthly target/,
  );
});

test('rejects unordered alerts and delayed protection stages', () => {
  const unorderedAlerts = validContract();
  unorderedAlerts.monthly.alertsCents = [1_500, 2_000, 1_900, 2_800, 3_000];
  assert.throws(
    () => validateBudgetContract(unorderedAlerts),
    /alerts must be strictly increasing/,
  );

  const lateFreeze = validContract();
  lateFreeze.stages[2].fromCents = 2_801;
  assert.throws(
    () => validateBudgetContract(lateFreeze),
    /freeze_cost_creation must start no later than 2800 cents/,
  );
});

test('selects the stage at every approved boundary', () => {
  const contract = validContract();

  assert.equal(stageForSpend(contract, 0), BudgetStage.Normal);
  assert.equal(stageForSpend(contract, 2_499), BudgetStage.Normal);
  assert.equal(stageForSpend(contract, 2_500), BudgetStage.DisableNonEssential);
  assert.equal(stageForSpend(contract, 2_800), BudgetStage.FreezeCostCreation);
  assert.equal(stageForSpend(contract, 3_000), BudgetStage.EssentialOnly);
  assert.equal(stageForSpend(contract, 9_999), BudgetStage.EssentialOnly);
});
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```bash
rtk node --test tools/finops/production-budget.test.mjs
```

Expected: FAIL with `ERR_MODULE_NOT_FOUND` for
`tools/finops/production-budget.mjs`.

- [ ] **Step 3: Implement the pure budget contract**

Create `tools/finops/production-budget.mjs`:

```javascript
import { readFile } from 'node:fs/promises';

export const BudgetStage = Object.freeze({
  Normal: 'normal',
  DisableNonEssential: 'disable_non_essential',
  FreezeCostCreation: 'freeze_cost_creation',
  EssentialOnly: 'essential_only',
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
  [BudgetStage.Normal, 0],
  [BudgetStage.DisableNonEssential, 2_500],
  [BudgetStage.FreezeCostCreation, 2_800],
  [BudgetStage.EssentialOnly, 3_000],
]);

function nonNegativeInteger(value) {
  return Number.isInteger(value) && value >= 0;
}

function strictlyIncreasing(values) {
  return values.every((value, index) => index === 0 || value > values[index - 1]);
}

export function validateBudgetContract(contract) {
  const errors = [];

  if (contract?.version !== 1) errors.push('version must equal 1');
  if (contract?.currency !== 'EUR') errors.push('currency must equal EUR');
  if (contract?.taxesIncluded !== true) errors.push('taxesIncluded must be true');

  const targetCents = contract?.monthly?.targetCents;
  const hardLimitCents = contract?.monthly?.hardLimitCents;
  const alertsCents = contract?.monthly?.alertsCents;

  if (!nonNegativeInteger(targetCents) || targetCents > 2_000) {
    errors.push('monthly target must not exceed 2000 cents');
  }
  if (!nonNegativeInteger(hardLimitCents) || hardLimitCents > 3_000) {
    errors.push('monthly hard limit must not exceed 3000 cents');
  }
  if (
    nonNegativeInteger(targetCents) &&
    nonNegativeInteger(hardLimitCents) &&
    hardLimitCents - targetCents < 1_000
  ) {
    errors.push('monthly reserve must be at least 1000 cents');
  }

  if (
    !Array.isArray(alertsCents) ||
    alertsCents.length === 0 ||
    !alertsCents.every(nonNegativeInteger) ||
    !strictlyIncreasing(alertsCents)
  ) {
    errors.push('alerts must be strictly increasing non-negative cents');
  } else if (alertsCents.at(-1) !== hardLimitCents) {
    errors.push('the final alert must equal the monthly hard limit');
  }

  const categories = contract?.categories;
  if (categories === null || typeof categories !== 'object' || Array.isArray(categories)) {
    errors.push('categories must be an object');
  } else {
    const keys = Object.keys(categories).sort();
    const expectedKeys = [...REQUIRED_CATEGORIES].sort();
    if (JSON.stringify(keys) !== JSON.stringify(expectedKeys)) {
      errors.push(`categories must be exactly: ${expectedKeys.join(', ')}`);
    }

    let allocatedTarget = 0;
    for (const name of REQUIRED_CATEGORIES) {
      const category = categories[name];
      if (
        !nonNegativeInteger(category?.targetCents) ||
        !nonNegativeInteger(category?.limitCents) ||
        category.targetCents > category.limitCents
      ) {
        errors.push(`${name} must have integer cents with targetCents <= limitCents`);
        continue;
      }
      allocatedTarget += category.targetCents;
    }
    if (nonNegativeInteger(targetCents) && allocatedTarget !== targetCents) {
      errors.push('category targets must equal the monthly target');
    }
  }

  const stages = contract?.stages;
  if (!Array.isArray(stages) || stages.length !== REQUIRED_STAGES.length) {
    errors.push('stages must define the four V1 budget stages');
  } else {
    for (const [index, [expectedName, latestStart]] of REQUIRED_STAGES.entries()) {
      const stage = stages[index];
      if (stage?.name !== expectedName) {
        errors.push(`stage ${index} must be ${expectedName}`);
      }
      if (!nonNegativeInteger(stage?.fromCents)) {
        errors.push(`${expectedName} fromCents must be a non-negative integer`);
      } else if (index === 0 && stage.fromCents !== 0) {
        errors.push('normal must start at 0 cents');
      } else if (index > 0 && stage.fromCents > latestStart) {
        errors.push(`${expectedName} must start no later than ${latestStart} cents`);
      }
    }

    const starts = stages.map((stage) => stage?.fromCents);
    if (!starts.every(nonNegativeInteger) || !strictlyIncreasing(starts)) {
      errors.push('budget stages must be strictly increasing');
    }
  }

  if (errors.length > 0) {
    throw new Error(`Invalid production FinOps budget:\n- ${errors.join('\n- ')}`);
  }

  return contract;
}

export function stageForSpend(contract, spendCents) {
  validateBudgetContract(contract);
  if (!nonNegativeInteger(spendCents)) {
    throw new TypeError('spendCents must be a non-negative integer');
  }

  let activeStage = contract.stages[0];
  for (const stage of contract.stages) {
    if (spendCents < stage.fromCents) break;
    activeStage = stage;
  }
  return activeStage.name;
}

export async function loadBudgetContract(file) {
  const source = await readFile(file, 'utf8');
  return validateBudgetContract(JSON.parse(source));
}
```

- [ ] **Step 4: Add the approved repository contract**

Create `infrastructure/finops/production-budget.json`:

```json
{
  "version": 1,
  "currency": "EUR",
  "taxesIncluded": true,
  "monthly": {
    "targetCents": 2000,
    "hardLimitCents": 3000,
    "alertsCents": [1500, 2000, 2500, 2800, 3000]
  },
  "stages": [
    { "name": "normal", "fromCents": 0 },
    { "name": "disable_non_essential", "fromCents": 2500 },
    { "name": "freeze_cost_creation", "fromCents": 2800 },
    { "name": "essential_only", "fromCents": 3000 }
  ],
  "categories": {
    "domain_dns": { "targetCents": 200, "limitCents": 300 },
    "compute": { "targetCents": 300, "limitCents": 600 },
    "postgres": { "targetCents": 400, "limitCents": 800 },
    "email": { "targetCents": 100, "limitCents": 200 },
    "storage_registry": { "targetCents": 200, "limitCents": 400 },
    "observability": { "targetCents": 0, "limitCents": 200 },
    "safety_margin": { "targetCents": 800, "limitCents": 800 }
  }
}
```

- [ ] **Step 5: Run the focused tests**

Run:

```bash
rtk node --test tools/finops/production-budget.test.mjs
```

Expected: 5 tests PASS.

- [ ] **Step 6: Commit the budget domain**

```bash
rtk git add infrastructure/finops/production-budget.json tools/finops/production-budget.mjs tools/finops/production-budget.test.mjs
rtk git commit -S -m "feat(finops): add production budget contract"
```

## Task 2: Wire the budget contract into the root checks

**Files:**

- Create: `tools/finops/check-production-budget.mjs`
- Modify: `package.json`

- [ ] **Step 1: Add the budget-check CLI**

Create `tools/finops/check-production-budget.mjs`:

```javascript
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
```

- [ ] **Step 2: Run the CLI against the repository contract**

Run:

```bash
rtk node tools/finops/check-production-budget.mjs
```

Expected:

```text
FinOps production budget passed: target=2000 cents, hard=3000 cents.
```

- [ ] **Step 3: Add the root scripts**

In `package.json`, replace the FinOps portion of `check` and add the two scripts:

```json
{
  "scripts": {
    "check": "pnpm env:check && pnpm check:structure && pnpm check:secrets && pnpm check:finops && pnpm check:web && pnpm check:api",
    "check:finops": "pnpm check:finops-production-budget && pnpm check:finops-scale-to-zero",
    "check:finops-production-budget": "node --test tools/finops/production-budget.test.mjs && node tools/finops/check-production-budget.mjs",
    "check:finops-scale-to-zero": "node --test tools/finops/check-scale-to-zero.test.mjs && node tools/finops/check-scale-to-zero.mjs"
  }
}
```

Preserve every unrelated script exactly as-is.

- [ ] **Step 4: Verify the new root target**

Run:

```bash
rtk pnpm check:finops-production-budget
```

Expected: 5 tests PASS and the repository budget contract passes.

- [ ] **Step 5: Commit the CI integration**

```bash
rtk git add package.json tools/finops/check-production-budget.mjs
rtk git commit -S -m "ci(finops): enforce production budget contract"
```

## Task 3: Reject unbounded Scaleway runtime maxima

**Files:**

- Modify: `tools/finops/check-scale-to-zero.test.mjs`
- Modify: `tools/finops/check-scale-to-zero.mjs`

- [ ] **Step 1: Replace the tests with minimum and maximum policy cases**

Replace `tools/finops/check-scale-to-zero.test.mjs` with:

```javascript
import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const checker = fileURLToPath(new URL('./check-scale-to-zero.mjs', import.meta.url));

async function runChecker(t, terraform) {
  const directory = await mkdtemp(path.join(tmpdir(), 'nvbes-scale-to-zero-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  await writeFile(path.join(directory, 'runtime.tf'), terraform);
  return spawnSync(process.execPath, [checker, directory], { encoding: 'utf8' });
}

test('accepts bounded zero-minimum Scaleway runtimes', async (t) => {
  const result = await runChecker(
    t,
    `
resource "scaleway_container" "api" {
  min_scale = 0
  max_scale = 1
}
resource "scaleway_sdb_sql_database" "api" {
  min_cpu = 0
  max_cpu = 1
}
`,
  );

  assert.equal(result.status, 0, result.stderr);
});

test('rejects non-zero, missing or excessive runtime bounds', async (t) => {
  const result = await runChecker(
    t,
    `
resource "scaleway_container" "always_on" {
  min_scale = 1
  max_scale = 1
}
resource "scaleway_container" "unbounded" {
  min_scale = 0
  max_scale = 10
}
resource "scaleway_sdb_sql_database" "missing_maximum" {
  min_cpu = 0
}
resource "scaleway_sdb_sql_database" "too_large" {
  min_cpu = 0
  max_cpu = 2
}
`,
  );

  assert.equal(result.status, 1);
  assert.match(result.stderr, /scaleway_container must declare min_scale = 0/);
  assert.match(result.stderr, /scaleway_container must declare max_scale as an integer <= 1/);
  assert.match(result.stderr, /scaleway_sdb_sql_database must declare max_cpu as an integer <= 1/);
});
```

- [ ] **Step 2: Run the tests and verify the maximum checks fail**

Run:

```bash
rtk node --test tools/finops/check-scale-to-zero.test.mjs
```

Expected: FAIL because the current checker does not report excessive or missing
maximum attributes.

- [ ] **Step 3: Replace the checker with literal min/max validation**

Replace `tools/finops/check-scale-to-zero.mjs` with:

```javascript
import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';

const ROOT = path.resolve(process.argv[2] ?? 'infrastructure');
const RESOURCE_POLICIES = new Map([
  [
    'scaleway_container',
    { minimumAttribute: 'min_scale', minimum: 0, maximumAttribute: 'max_scale', maximum: 1 },
  ],
  [
    'scaleway_sdb_sql_database',
    { minimumAttribute: 'min_cpu', minimum: 0, maximumAttribute: 'max_cpu', maximum: 1 },
  ],
]);

async function terraformFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    if (entry.name === '.terraform') continue;
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await terraformFiles(entryPath)));
    else if (entry.isFile() && entry.name.endsWith('.tf')) files.push(entryPath);
  }

  return files;
}

function resourceBlocks(source, resourceType) {
  const pattern = new RegExp(`resource\\s+"${resourceType}"\\s+"[^"]+"\\s*\\{`, 'g');
  const blocks = [];

  for (const match of source.matchAll(pattern)) {
    let depth = 1;
    let cursor = match.index + match[0].length;
    while (cursor < source.length && depth > 0) {
      if (source[cursor] === '{') depth += 1;
      if (source[cursor] === '}') depth -= 1;
      cursor += 1;
    }
    blocks.push(source.slice(match.index, cursor));
  }

  return blocks;
}

function integerAttribute(block, attribute) {
  const match = new RegExp(`\\b${attribute}\\s*=\\s*(-?\\d+)\\b`).exec(block);
  return match ? Number.parseInt(match[1], 10) : undefined;
}

const violations = [];

for (const file of await terraformFiles(ROOT)) {
  const source = await readFile(file, 'utf8');
  for (const [resourceType, policy] of RESOURCE_POLICIES) {
    for (const block of resourceBlocks(source, resourceType)) {
      const minimum = integerAttribute(block, policy.minimumAttribute);
      const maximum = integerAttribute(block, policy.maximumAttribute);
      const relativeFile = path.relative(process.cwd(), file);

      if (minimum !== policy.minimum) {
        violations.push(
          `${relativeFile}: ${resourceType} must declare ${policy.minimumAttribute} = ${policy.minimum}`,
        );
      }
      if (maximum === undefined || maximum > policy.maximum) {
        violations.push(
          `${relativeFile}: ${resourceType} must declare ${policy.maximumAttribute} as an integer <= ${policy.maximum}`,
        );
      }
    }
  }
}

if (violations.length > 0) {
  console.error(violations.join('\n'));
  process.exitCode = 1;
} else {
  console.log('FinOps scale bounds passed for all Scaleway runtimes.');
}
```

- [ ] **Step 4: Verify unit tests pass and repository infrastructure fails**

Run:

```bash
rtk node --test tools/finops/check-scale-to-zero.test.mjs
rtk node tools/finops/check-scale-to-zero.mjs
```

Expected: 2 tests PASS, then the repository check FAILS for Email
`max_scale = 10`, Trust/Risk `max_scale = 10`, and Trust/Risk `max_cpu = 2`.

- [ ] **Step 5: Commit the stricter checker before changing infrastructure**

```bash
rtk git add tools/finops/check-scale-to-zero.mjs tools/finops/check-scale-to-zero.test.mjs
rtk git commit -S -m "ci(finops): reject unbounded serverless scaling"
```

## Task 4: Bring active production runtimes under the V1 ceilings

**Files:**

- Modify: `infrastructure/environments/email-production/email-delivery.tf`
- Modify: `infrastructure/environments/trust-risk-production/runtime.tf`
- Modify: `infrastructure/environments/trust-risk-production/database.tf`

- [ ] **Step 1: Cap the Email runtime**

In `scaleway_container.email_runtime`, change only the maximum:

```hcl
  min_scale = 0
  max_scale = 1
```

- [ ] **Step 2: Cap the Trust/Risk runtime and database**

In `scaleway_container.trust_risk`:

```hcl
  min_scale = 0
  max_scale = 1
```

In `scaleway_sdb_sql_database.trust_risk`:

```hcl
  min_cpu = 0
  max_cpu = 1
```

- [ ] **Step 3: Format the affected Terraform**

Run:

```bash
rtk terraform fmt infrastructure/environments/email-production/email-delivery.tf infrastructure/environments/trust-risk-production/runtime.tf infrastructure/environments/trust-risk-production/database.tf
```

Expected: Terraform formats the files without error.

- [ ] **Step 4: Run the complete static FinOps gate**

Run:

```bash
rtk pnpm check:finops
```

Expected: budget tests, budget contract, scale-bound tests and repository
Terraform checks all PASS.

- [ ] **Step 5: Commit the bounded runtimes**

```bash
rtk git add infrastructure/environments/email-production/email-delivery.tf infrastructure/environments/trust-risk-production/runtime.tf infrastructure/environments/trust-risk-production/database.tf
rtk git commit -S -m "fix(finops): cap production serverless scaling"
```

## Task 5: Add the reusable Rust budget-stage primitive

**Files:**

- Create: `libs/rust/platform/src/platform.finops.budget.rs`
- Create: `libs/rust/platform/src/platform.finops.budget.tests.rs`
- Modify: `libs/rust/platform/src/lib.rs`

- [ ] **Step 1: Write the failing Rust tests and declare the module**

Create `libs/rust/platform/src/platform.finops.budget.tests.rs`:

```rust
use super::{BudgetPolicyError, BudgetStage, BudgetThresholds};

#[test]
fn selects_every_v1_budget_stage_at_its_boundary() {
    let thresholds = BudgetThresholds::try_new(2_500, 2_800, 3_000)
        .expect("approved thresholds must be valid");

    assert_eq!(thresholds.stage_for(0), BudgetStage::Normal);
    assert_eq!(thresholds.stage_for(2_499), BudgetStage::Normal);
    assert_eq!(
        thresholds.stage_for(2_500),
        BudgetStage::DisableNonEssential
    );
    assert_eq!(
        thresholds.stage_for(2_800),
        BudgetStage::FreezeCostCreation
    );
    assert_eq!(thresholds.stage_for(3_000), BudgetStage::EssentialOnly);
    assert_eq!(thresholds.stage_for(8_000), BudgetStage::EssentialOnly);
}

#[test]
fn rejects_zero_or_unordered_thresholds() {
    assert_eq!(
        BudgetThresholds::try_new(0, 2_800, 3_000),
        Err(BudgetPolicyError::ZeroThreshold)
    );
    assert_eq!(
        BudgetThresholds::try_new(2_800, 2_800, 3_000),
        Err(BudgetPolicyError::UnorderedThresholds)
    );
    assert_eq!(
        BudgetThresholds::try_new(2_500, 3_100, 3_000),
        Err(BudgetPolicyError::UnorderedThresholds)
    );
}

#[test]
fn stage_names_match_the_repository_contract() {
    assert_eq!(
        serde_json::to_string(&BudgetStage::DisableNonEssential)
            .expect("stage must serialize"),
        "\"disable_non_essential\""
    );
    assert_eq!(
        serde_json::from_str::<BudgetStage>("\"freeze_cost_creation\"")
            .expect("stage must deserialize"),
        BudgetStage::FreezeCostCreation
    );
}
```

Create `libs/rust/platform/src/platform.finops.budget.rs` with only the test
module declaration:

```rust
#[cfg(test)]
#[path = "platform.finops.budget.tests.rs"]
mod tests;
```

Add the module to `libs/rust/platform/src/lib.rs`:

```rust
#[path = "platform.finops.budget.rs"]
pub mod finops_budget;
```

- [ ] **Step 2: Run the platform tests and verify compilation fails**

Run:

```bash
rtk pnpm nx run rust-workspace:test
```

Expected: FAIL with unresolved imports for `BudgetPolicyError`, `BudgetStage`
and `BudgetThresholds`.

- [ ] **Step 3: Implement the pure stage selector**

Replace `libs/rust/platform/src/platform.finops.budget.rs` with:

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStage {
    Normal,
    DisableNonEssential,
    FreezeCostCreation,
    EssentialOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetThresholds {
    disable_non_essential_cents: u32,
    freeze_cost_creation_cents: u32,
    essential_only_cents: u32,
}

#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum BudgetPolicyError {
    #[error("budget thresholds must be greater than zero")]
    ZeroThreshold,
    #[error("budget thresholds must be strictly increasing")]
    UnorderedThresholds,
}

impl BudgetThresholds {
    pub fn try_new(
        disable_non_essential_cents: u32,
        freeze_cost_creation_cents: u32,
        essential_only_cents: u32,
    ) -> Result<Self, BudgetPolicyError> {
        if disable_non_essential_cents == 0
            || freeze_cost_creation_cents == 0
            || essential_only_cents == 0
        {
            return Err(BudgetPolicyError::ZeroThreshold);
        }
        if disable_non_essential_cents >= freeze_cost_creation_cents
            || freeze_cost_creation_cents >= essential_only_cents
        {
            return Err(BudgetPolicyError::UnorderedThresholds);
        }

        Ok(Self {
            disable_non_essential_cents,
            freeze_cost_creation_cents,
            essential_only_cents,
        })
    }

    pub fn stage_for(self, spend_cents: u32) -> BudgetStage {
        if spend_cents >= self.essential_only_cents {
            BudgetStage::EssentialOnly
        } else if spend_cents >= self.freeze_cost_creation_cents {
            BudgetStage::FreezeCostCreation
        } else if spend_cents >= self.disable_non_essential_cents {
            BudgetStage::DisableNonEssential
        } else {
            BudgetStage::Normal
        }
    }
}

#[cfg(test)]
#[path = "platform.finops.budget.tests.rs"]
mod tests;
```

At the end of `libs/rust/platform/src/lib.rs`, add explicit re-exports:

```rust
pub use finops_budget::{BudgetPolicyError, BudgetStage, BudgetThresholds};
```

- [ ] **Step 4: Run focused and workspace tests**

Run:

```bash
rtk pnpm nx run rust-workspace:test
```

Expected: 3 new platform tests and the complete locked Cargo workspace test
target PASS.

- [ ] **Step 5: Run the mandatory Rust workspace check**

Run:

```bash
rtk pnpm nx run rust-workspace:check
```

Expected: `cargo check --workspace --locked` PASS with no warnings introduced by
the new module.

- [ ] **Step 6: Commit the runtime primitive**

```bash
rtk git add libs/rust/platform/src/lib.rs libs/rust/platform/src/platform.finops.budget.rs libs/rust/platform/src/platform.finops.budget.tests.rs
rtk git commit -S -m "feat(platform): add FinOps budget stages"
```

## Task 6: Document operation and run the release gate

**Files:**

- Create: `infrastructure/finops/README.md`
- Modify: `infrastructure/README.md`

- [ ] **Step 1: Add the FinOps operating procedure**

Create `infrastructure/finops/README.md`:

```markdown
# Production FinOps contract

`production-budget.json` is the executable tax-inclusive monthly budget for the
nvbes V1 production foundation.

The target is 20 EUR and the hard limit is 30 EUR, including compute, databases,
email, storage, observability, domains, taxes and every recurring provider.

## Change procedure

1. Record the latest tax-inclusive invoice or provider estimate.
2. Update the relevant category without raising the 20 EUR target or 30 EUR hard
   limit.
3. Run `pnpm check:finops`.
4. Include the measured reason and affected cost unit in the commit message or
   pull-request description.

Changing the approved ceilings requires a new design decision. Provider alerts
do not replace repository limits.

## Degradation stages

- `normal`: all budgeted V1 work is available.
- `disable_non_essential`: optional processing is disabled from 25 EUR.
- `freeze_cost_creation`: new cost-creating operations are frozen from 28 EUR.
- `essential_only`: only recovery, critical email and durable provider webhook
  ingestion remain from 30 EUR.

Platform Operations will consume the shared Rust stage selector when live spend
collection is implemented. Until that runtime exists, literal Terraform maxima
and provider-side alerts are the automatic outer guardrails.
```

- [ ] **Step 2: Link the contract from the infrastructure overview**

Add this paragraph after the provider list in `infrastructure/README.md`:

```markdown
## FinOps gate

The executable V1 production budget and its operating procedure live in
[`finops/`](finops/README.md). Run `pnpm check:finops` before applying any
production infrastructure change.
```

- [ ] **Step 3: Run formatting and focused checks**

Run:

```bash
rtk terraform fmt -check -recursive infrastructure/environments/email-production infrastructure/environments/trust-risk-production
rtk pnpm check:finops
rtk cargo fmt --all --check
rtk git diff --check
```

Expected: all four commands PASS.

- [ ] **Step 4: Run the mandatory Rust validation**

Run:

```bash
rtk pnpm nx run rust-workspace:check
rtk pnpm nx run rust-workspace:test
```

Expected: both Nx targets PASS.

- [ ] **Step 5: Commit documentation**

```bash
rtk git add infrastructure/README.md infrastructure/finops/README.md
rtk git commit -S -m "docs(finops): document V1 budget operations"
```

- [ ] **Step 6: Verify the completed slice**

Run:

```bash
rtk git status --short
rtk pnpm check:finops
```

Expected: the worktree is clean and the full FinOps gate passes. Do not start an
Identity, Account, Billing or Platform Operations runtime in this plan.

## Completion evidence

This plan is complete only when all of the following are true:

- the repository rejects a target above 20 EUR or a hard limit above 30 EUR;
- the budget contract is tax-inclusive and category targets reconcile exactly;
- active Scaleway containers declare `min_scale = 0` and `max_scale <= 1`;
- active Serverless SQL databases declare `min_cpu = 0` and `max_cpu <= 1`;
- Rust consumers can select the four approved degradation stages from validated
  thresholds;
- the root `check` command includes `check:finops`;
- the operating procedure explains how measured costs update the contract;
- all commits are signed and the final worktree is clean.
