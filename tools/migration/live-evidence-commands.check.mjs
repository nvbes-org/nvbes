#!/usr/bin/env node
import { buildPreparationCommands } from "./live-evidence-commands.mjs";
import { buildLiveEvidenceTemplate, requiredEvidenceFor } from "./live-evidence.rules.mjs";

const requirements = [
	{ id: "g4-frontend-signoff", expected_evidence: requiredEvidenceFor("g4-frontend-signoff") },
	{ id: "g5-infra-signoff", expected_evidence: requiredEvidenceFor("g5-infra-signoff") },
	{ id: "p11-rehearsals", expected_evidence: requiredEvidenceFor("p11-rehearsals") },
	{ id: "p12-cutover", expected_evidence: requiredEvidenceFor("p12-cutover") },
	{ id: "p13-decommission", expected_evidence: requiredEvidenceFor("p13-decommission") },
	{ id: "final-reconciliation", expected_evidence: requiredEvidenceFor("final-reconciliation") },
];

const commands = buildPreparationCommands(requirements);
const errors = [];
const ids = new Set();
const outputs = new Set();
const immutableReferences = new Set();
const externalSourceArtifacts = new Set();

if (commands.length !== 17) errors.push(`expected 17 preparation commands, got ${commands.length}`);
for (const command of commands) {
	const tokens = tokenize(command);
	if (tokens[0] !== "node" || tokens[1] !== "tools/migration/live-evidence-prepare.mjs") {
		errors.push(`command must invoke live-evidence-prepare.mjs: ${command}`);
	}
	const values = optionValues(tokens);
	checkUnique(ids, values.get("--id"), "--id");
	checkUnique(outputs, values.get("--out"), "--out");
	checkUnique(immutableReferences, values.get("--immutable-reference"), "--immutable-reference");
	if (values.get("--source-artifact")?.includes("://")) {
		checkUnique(externalSourceArtifacts, values.get("--source-artifact"), "--source-artifact");
		if (!values.has("--source-checksum")) errors.push(`${values.get("--id")}: external source artifact requires --source-checksum`);
	}
	if (values.get("--out") !== `docs/migration/live-evidence-instances/${values.get("--id")}.json`) {
		errors.push(`${values.get("--id")}: --out must match evidence id`);
	}
	for (const required of ["--type", "--env", "--owner", "--source-artifact", "--command", "--result", "--decision", "--packet-requirement", "--notes"]) {
		if (!values.has(required)) errors.push(`${values.get("--id") ?? command}: missing ${required}`);
	}
	if (values.get("--result") !== "passed" || values.get("--decision") !== "go") {
		errors.push(`${values.get("--id")}: generated command must make passed/go explicit`);
	}
	if (values.get("--command")?.includes("--report") && values.get("--source-artifact") !== reportPath(values.get("--command"))) {
		errors.push(`${values.get("--id")}: report command source artifact must match --report`);
	}
}

const templateNotes = buildLiveEvidenceTemplate().notes;
if (!templateNotes.includes("live-evidence-prepare.mjs")) {
	errors.push("live evidence template notes must point operators to live-evidence-prepare.mjs");
}

function tokenize(command) {
	const tokens = [];
	const pattern = /'((?:[^']|'"\''"')*)'|"([^"]*)"|(\S+)/g;
	for (const match of command.matchAll(pattern)) {
		tokens.push((match[1] ?? match[2] ?? match[3]).replaceAll("'\"'\"'", "'"));
	}
	return tokens;
}

function optionValues(tokens) {
	const values = new Map();
	for (let index = 0; index < tokens.length; index += 1) {
		const token = tokens[index];
		if (!token.startsWith("--")) continue;
		values.set(token, tokens[index + 1] ?? "");
		index += 1;
	}
	return values;
}

function checkUnique(seen, value, label) {
	if (!value) {
		errors.push(`${label}: missing value`);
		return;
	}
	if (seen.has(value)) errors.push(`${label}: duplicate ${value}`);
	seen.add(value);
}

function reportPath(command) {
	const tokens = tokenize(command);
	const index = tokens.indexOf("--report");
	return index >= 0 ? tokens[index + 1] : tokens.find((token) => token.startsWith("--report="))?.slice("--report=".length);
}

if (errors.length > 0) {
	console.error("Live evidence command checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Live evidence commands: ok (${commands.length} commands)`);
