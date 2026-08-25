import assert from "node:assert/strict";
import test from "node:test";

import {
	BudgetStage,
	loadBudgetContract,
	stageForSpend,
	validateBudgetContract,
} from "./production-budget.mjs";

function validContract() {
	return {
		version: 1,
		currency: "EUR",
		taxesIncluded: true,
		monthly: {
			targetCents: 2000,
			hardLimitCents: 3000,
			alertsCents: [1500, 2000, 2500, 2800, 3000],
		},
		categories: {
			domain_dns: { targetCents: 200, limitCents: 300 },
			compute: { targetCents: 300, limitCents: 600 },
			postgres: { targetCents: 400, limitCents: 800 },
			email: { targetCents: 100, limitCents: 200 },
			storage_registry: { targetCents: 200, limitCents: 400 },
			observability: { targetCents: 0, limitCents: 200 },
			safety_margin: { targetCents: 800, limitCents: 800 },
		},
		stages: [
			{ name: BudgetStage.Normal, fromCents: 0 },
			{ name: BudgetStage.DisableNonEssential, fromCents: 2500 },
			{ name: BudgetStage.FreezeCostCreation, fromCents: 2800 },
			{ name: BudgetStage.EssentialOnly, fromCents: 3000 },
		],
	};
}

test("accepts the approved production budget contract", () => {
	const contract = validContract();

	assert.equal(validateBudgetContract(contract), contract);
});

test("rejects a monthly target above 2000 cents", () => {
	const contract = validContract();
	contract.monthly.targetCents = 2001;

	assert.throws(
		() => validateBudgetContract(contract),
		/monthly target must not exceed 2000 cents/,
	);
});

test("rejects a monthly hard limit above 3000 cents", () => {
	const contract = validContract();
	contract.monthly.hardLimitCents = 3001;

	assert.throws(
		() => validateBudgetContract(contract),
		/monthly hard limit must not exceed 3000 cents/,
	);
});

test("rejects a contract without the 1000-cent reserve", () => {
	const contract = validContract();
	contract.monthly.targetCents = 2500;

	assert.throws(
		() => validateBudgetContract(contract),
		/reserve must be at least 1000 cents/,
	);
});

test("rejects category target sum drift", () => {
	const contract = validContract();
	contract.categories.compute.targetCents = 301;

	assert.throws(
		() => validateBudgetContract(contract),
		/category target sum must equal monthly target/,
	);
});

test("rejects unordered alerts", () => {
	const contract = validContract();
	contract.monthly.alertsCents = [1500, 2500, 2000, 2800, 3000];

	assert.throws(
		() => validateBudgetContract(contract),
		/alerts must be strictly increasing/,
	);
});

for (const requiredAlertCents of [1500, 2000, 2500]) {
	test(`rejects a contract missing the required ${requiredAlertCents}-cent alert`, () => {
		const contract = validContract();
		contract.monthly.alertsCents = contract.monthly.alertsCents.filter(
			(alert) => alert !== requiredAlertCents,
		);

		assert.throws(
			() => validateBudgetContract(contract),
			new Error(
				`Invalid production FinOps budget: alerts must include the required ${requiredAlertCents}-cent threshold`,
			),
		);
	});
}

test("rejects a hard-limit-only alert schedule", () => {
	const contract = validContract();
	contract.monthly.alertsCents = [3000];

	assert.throws(
		() => validateBudgetContract(contract),
		/alerts must include the required 1500-cent threshold/,
	);
});

test("accepts an additional strictly increasing alert", () => {
	const contract = validContract();
	contract.monthly.alertsCents = [1500, 1750, 2000, 2500, 2800, 3000];

	assert.equal(validateBudgetContract(contract), contract);
});

test("rejects freeze_cost_creation later than 2800 cents", () => {
	const contract = validContract();
	contract.stages[2].fromCents = 2801;

	assert.throws(
		() => validateBudgetContract(contract),
		/freeze_cost_creation stage must start no later than 2800 cents/,
	);
});

test("selects stages at inclusive spend boundaries and above the hard limit", () => {
	const contract = validContract();

	assert.equal(stageForSpend(contract, 0), BudgetStage.Normal);
	assert.equal(stageForSpend(contract, 2499), BudgetStage.Normal);
	assert.equal(stageForSpend(contract, 2500), BudgetStage.DisableNonEssential);
	assert.equal(stageForSpend(contract, 2800), BudgetStage.FreezeCostCreation);
	assert.equal(stageForSpend(contract, 3000), BudgetStage.EssentialOnly);
	assert.equal(stageForSpend(contract, 3001), BudgetStage.EssentialOnly);
});

test("loads the pinned approved production budget fixture", async () => {
	const contract = await loadBudgetContract(
		new URL(
			"../../infrastructure/finops/production-budget.json",
			import.meta.url,
		),
	);

	assert.deepEqual(contract.categories, {
		domain_dns: { targetCents: 200, limitCents: 300 },
		compute: { targetCents: 300, limitCents: 600 },
		postgres: { targetCents: 400, limitCents: 800 },
		email: { targetCents: 100, limitCents: 200 },
		storage_registry: { targetCents: 200, limitCents: 400 },
		observability: { targetCents: 0, limitCents: 200 },
		safety_margin: { targetCents: 800, limitCents: 800 },
	});
});
