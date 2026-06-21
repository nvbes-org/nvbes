#!/usr/bin/env node
import { optionValue } from "./cli-options.mjs";

const errors = [];

function expectValue(label, args, flag, expected) {
	const actual = optionValue(args, flag);
	if (actual !== expected) errors.push(`${label}: expected ${flag}=${expected}, got ${actual}`);
}

function expectFallback(label, args, flag, fallback) {
	const actual = optionValue(args, flag, fallback);
	if (actual !== fallback) errors.push(`${label}: expected fallback ${fallback}, got ${actual}`);
}

expectValue("space separated env", ["--env", "production"], "--env", "production");
expectValue("inline env", ["--env=staging"], "--env", "staging");
expectValue(
	"space separated report",
	["--report", "docs/migration/reconciliation.run.json"],
	"--report",
	"docs/migration/reconciliation.run.json",
);
expectValue(
	"inline report",
	["--report=docs/migration/reconciliation.run.json"],
	"--report",
	"docs/migration/reconciliation.run.json",
);
expectFallback("missing option", ["--env", "local"], "--report", "missing");

if (errors.length > 0) {
	console.error("Migration CLI option checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log("Migration CLI options: ok");
