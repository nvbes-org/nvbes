import { existsSync } from "node:fs";

export function validateInventory(value, outputPath) {
	const errors = [];
	if (value.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (value.generation?.command !== "node tools/migration/inventory.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (value.generation?.deterministic !== true) {
		errors.push(`${outputPath}: generation.deterministic must be true`);
	}
	validateSummary(value, outputPath, errors);
	validateCollection(value.routes, "routes", (entry) => entry.file, outputPath, errors, validateRouteEntry);
	validateCollection(value.tables, "tables", (entry) => entry.file, outputPath, errors, validateTableEntry);
	validateCollection(value.jobs, "jobs", (entry) => entry.file, outputPath, errors, validateJobEntry);
	validateStringList(value.infrastructure, "infrastructure", outputPath, errors);
	validateStringList(value.docs, "docs", outputPath, errors);
	validateCollection(value.secrets, "secrets", (entry) => entry.file, outputPath, errors, validateSecretEntry);
	validateCollection(value.resources, "resources", resourceKey, outputPath, errors, validateResourceEntry);
	return errors;
}

function validateSummary(value, outputPath, errors) {
	const expected = {
		route_files: value.routes.length,
		endpoints: value.routes.reduce((sum, entry) => sum + entry.endpoints.length, 0),
		migration_files: value.tables.length,
		tables: value.tables.reduce((sum, entry) => sum + entry.tables.length, 0),
		job_files: value.jobs.length,
		job_references: value.jobs.reduce((sum, entry) => sum + entry.constants.length + entry.jobTypes.length, 0),
		infrastructure_files: value.infrastructure.length,
		doc_files: value.docs.length,
		secret_files: value.secrets.length,
		secret_keys: value.secrets.reduce((sum, entry) => sum + entry.keys.length, 0),
		resources: value.resources.length,
	};
	for (const [key, count] of Object.entries(expected)) {
		if (value.summary[key] !== count) errors.push(`${outputPath}: summary ${key} must match inventory rows`);
	}
}

function validateCollection(entries, label, keyForEntry, outputPath, errors, validateEntry) {
	if (!Array.isArray(entries)) return errors.push(`${outputPath}: ${label} must be an array`);
	const seen = new Set();
	for (const entry of entries) {
		const key = keyForEntry(entry);
		if (!key) errors.push(`${outputPath}: ${label} entry key is required`);
		if (seen.has(key)) errors.push(`${outputPath}: duplicate ${label} entry ${key}`);
		seen.add(key);
		validateEntry(entry, outputPath, errors);
	}
}

function validateStringList(values, label, outputPath, errors) {
	if (!Array.isArray(values)) return errors.push(`${outputPath}: ${label} must be an array`);
	const seen = new Set();
	for (const value of values) {
		if (typeof value !== "string" || !value) errors.push(`${outputPath}: ${label} value is required`);
		if (seen.has(value)) errors.push(`${outputPath}: duplicate ${label} value ${value}`);
		seen.add(value);
		if (!existsSync(value)) errors.push(`${outputPath}: ${label} file is missing: ${value}`);
	}
}

function validateRouteEntry(entry, outputPath, errors) {
	if (!entry.file) errors.push(`${outputPath}: route file is required`);
	else if (!existsSync(entry.file)) errors.push(`${outputPath}: route file is missing: ${entry.file}`);
	validateNonEmptyStrings(entry.endpoints, `${entry.file}: endpoints`, errors);
}

function validateTableEntry(entry, outputPath, errors) {
	if (!entry.file) errors.push(`${outputPath}: table file is required`);
	else if (!existsSync(entry.file)) errors.push(`${outputPath}: table file is missing: ${entry.file}`);
	validateNonEmptyStrings(entry.tables, `${entry.file}: tables`, errors);
}

function validateJobEntry(entry, outputPath, errors) {
	if (!entry.file) errors.push(`${outputPath}: job file is required`);
	else if (!existsSync(entry.file)) errors.push(`${outputPath}: job file is missing: ${entry.file}`);
	if (!Array.isArray(entry.constants)) errors.push(`${entry.file}: constants must be an array`);
	if (!Array.isArray(entry.jobTypes)) errors.push(`${entry.file}: jobTypes must be an array`);
	if ((entry.constants?.length ?? 0) + (entry.jobTypes?.length ?? 0) === 0) {
		errors.push(`${entry.file}: job entry must include constants or jobTypes`);
	}
}

function validateSecretEntry(entry, outputPath, errors) {
	if (!entry.file) errors.push(`${outputPath}: secret file is required`);
	else if (!existsSync(entry.file)) errors.push(`${outputPath}: secret file is missing: ${entry.file}`);
	validateNonEmptyStrings(entry.keys, `${entry.file}: keys`, errors);
}

function validateResourceEntry(entry, _outputPath, errors) {
	for (const field of ["type", "source", "name"]) {
		if (!entry[field]) errors.push(`${resourceKey(entry)}: resource ${field} is required`);
	}
}

function validateNonEmptyStrings(values, label, errors) {
	if (!Array.isArray(values) || values.length === 0) return errors.push(`${label} must be a non-empty array`);
	const seen = new Set();
	for (const value of values) {
		if (typeof value !== "string" || !value) errors.push(`${label} contains an empty value`);
		if (seen.has(value)) errors.push(`${label} contains duplicate value ${value}`);
		seen.add(value);
	}
}

function resourceKey(entry) {
	return `${entry.type}:${entry.name}:${entry.source}`;
}
