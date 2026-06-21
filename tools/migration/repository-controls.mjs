#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { readPackageScripts } from "./execution-backlog.proof.mjs";

const verbose = process.argv.includes("--verbose");
const maxFailureLines = 25;
const failures = [];

const controls = [
	["contracts", "check:contracts"],
	["codegen", "check:codegen"],
	["product boundaries", "check:product-boundaries"],
	["secrets", "check:secrets"],
	["supply chain", "check:supply-chain"],
	["OSS boundaries", "check:oss-boundaries"],
];

function packageScripts() {
	return readPackageScripts(failures);
}

function validateControls() {
	const scripts = packageScripts();
	const seenLabels = new Set();
	const seenScripts = new Set();
	for (const [label, script] of controls) {
		if (!label) failures.push("repository control label is required");
		if (!script) failures.push(`${label}: repository control script is required`);
		if (seenLabels.has(label)) failures.push(`duplicate repository control label ${label}`);
		if (seenScripts.has(script)) failures.push(`duplicate repository control script ${script}`);
		seenLabels.add(label);
		seenScripts.add(script);
		if (!scripts[script]) failures.push(`${label}: package script ${script} is missing`);
	}
	const requiredScripts = [
		"check:contracts",
		"check:codegen",
		"check:product-boundaries",
		"check:secrets",
		"check:supply-chain",
		"check:oss-boundaries",
	];
	for (const script of requiredScripts) {
		if (!seenScripts.has(script)) failures.push(`repository controls missing ${script}`);
	}
}

validateControls();
if (failures.length > 0) {
	console.error("Migration repository controls failed:");
	for (const failure of failures) console.error(`- ${failure}`);
	process.exit(1);
}

for (const [label, script] of controls) {
	const result = spawnSync("pnpm", [script], {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});

	if (result.status === 0) {
		const line = result.stdout.trim().split("\n").at(-1);
		console.log(`ok: ${label}${line ? ` (${line})` : ""}`);
		continue;
	}

	failures.push(`${label} failed`);
	const output = `${result.stdout}${result.stderr}`.trim();
	if (!output) continue;
	const lines = output.split("\n");
	const visibleLines = verbose ? lines : lines.slice(0, maxFailureLines);
	for (const line of visibleLines) console.error(`${label}: ${line}`);
	if (!verbose && lines.length > maxFailureLines) {
		console.error(
			`${label}: ... ${lines.length - maxFailureLines} more lines omitted; rerun with --verbose for full output`,
		);
	}
}

if (failures.length > 0) {
	console.error("Migration repository controls failed:");
	for (const failure of failures) console.error(`- ${failure}`);
	process.exit(1);
}

console.log(`Migration repository controls: ok (${controls.length} controls)`);
