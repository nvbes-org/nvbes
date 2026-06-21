export function validateDecisionMapRows({ map, expectedEntries, keyFor, outputPath, errors }) {
	const entries = Array.isArray(map.entries) ? map.entries : [];
	const expectedByKey = new Map(expectedEntries.map((entry) => [keyFor(entry), entry]));
	const seen = new Set();
	validateSummary(map, entries, outputPath, errors);
	for (const entry of entries) {
		const key = keyFor(entry);
		seen.add(key);
		const expected = expectedByKey.get(key);
		if (!expected) {
			errors.push(`${key}: entry must exist in inventory-derived source set`);
			continue;
		}
		if (JSON.stringify(entry.source) !== JSON.stringify(expected.source)) {
			errors.push(`${key}: source must match inventory`);
		}
		if (entry.domain !== expected.domain) errors.push(`${key}: domain must match inventory inference`);
	}
	for (const key of expectedByKey.keys()) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing inventory-derived entry ${key}`);
	}
}

function validateSummary(map, entries, outputPath, errors) {
	const summary = map.summary ?? {};
	const counts = { pending: 0, keep: 0, rebuild: 0, remove: 0, replace: 0, rotate: 0 };
	for (const entry of entries) {
		if (entry.decision in counts) counts[entry.decision] += 1;
	}
	if (summary.entries !== entries.length) errors.push(`${outputPath}: summary entries must match rows`);
	for (const key of Object.keys(summary)) {
		if (key === "entries") continue;
		if (key in counts && summary[key] !== counts[key]) {
			errors.push(`${outputPath}: summary ${key} must match rows`);
		}
	}
}
