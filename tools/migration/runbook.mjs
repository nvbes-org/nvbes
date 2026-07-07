#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { requirements as cutoverRequirements, strictCommandFor } from "./cutover-evidence-packet.requirements.mjs";
import { readPackageScripts } from "./execution-backlog.proof.mjs";

const errors = [];
const path = "docs/migration/nvbes-big-bang-migration-runbook.md";
const content = readFileSync(path, "utf8");
const lines = content.split("\n");
const lineCount = content.endsWith("\n") ? lines.length - 1 : lines.length;
const migrationScripts = Object.keys(readPackageScripts(errors))
	.filter((script) => script.startsWith("check:migration-"))
	.sort();
const completionRequirementsPath = "tools/migration/completion-requirements.mjs";
const completionRequirements = readFileSync(completionRequirementsPath, "utf8");

const requiredHeadings = [
	"# Runbook Migration Big Bang nvbes",
	"## Objectif",
	"## Principes d'Execution",
	"## Statut Operationnel",
	"## Roles",
	"## Artefacts Obligatoires",
	"## Commandes de Validation",
	"## Seuils Go/No-Go",
	"## Phase 0 - Freeze et Inventaire",
	"## Phase 1 - Fondation Parallele",
	"## Phase 2 - Primitives Plateforme",
	"## Phase 3 - Migration Domaines",
	"## Phase 4 - Frontends",
	"## Phase 5 - Infra et Operations",
	"## Phase 6 - Repetitions Migration",
	"## Phase 7 - Cutover Big Bang",
	"## Rollback",
	"## Reconciliation",
	"## Communication",
	"## Phase 8 - Decommission",
	"## Validation Globale",
];

const requiredArtifacts = [
	"inventory.md",
	"parity-matrix.md",
	"owner-signoff-matrix.md",
	"data-map.md",
	"snapshot-manifest.md",
	"reconciliation-report.md",
	"rehearsal-ledger.md",
	"rejects.md",
	"cutover-checklist.md",
	"live-evidence-instances/",
	"live-evidence.template.json",
	"communication-plan.md",
	"cutover-journal.md",
	"gate-evidence.md",
	"rollback-report.md",
	"risk-register.md",
	"post-migration-audit.md",
	"decommission-manifest.md",
	"v2-debt-register.md",
];

const requiredCommands = [
	"pnpm migration:generate",
	"pnpm check:migration-blueprint",
	"pnpm check:migration-runbook",
	"pnpm check:migration-inventory",
	"pnpm check:migration-data-map",
	"pnpm check:migration-secret-map",
	"pnpm check:migration-job-map",
	"pnpm check:migration-resource-map",
	"pnpm check:migration-runtime-foundation",
	"pnpm check:migration-phase-ledger",
	"pnpm check:migration-domain-ledger",
	"pnpm check:migration-domain-dod",
	"pnpm check:migration-risk-register",
	"pnpm check:migration-gate-evidence",
	"pnpm check:migration-readiness-report",
	"pnpm check:migration-completion-audit",
	"pnpm check:migration-v2-debt",
	"pnpm check:migration-execution-backlog",
	"pnpm check:migration-precutover",
	"pnpm check:migration-postcutover",
	"pnpm check:secrets",
	"pnpm check:codegen",
	"pnpm check:supply-chain",
	"pnpm check:go",
	"pnpm check:python",
	"node tools/migration/reconcile.mjs --env staging",
];

const requiredCutoverSteps = [
	"1. annoncer maintenance",
	"2. passer l'ancien systeme en read-only",
	"3. stopper les workers anciens",
	"4. prendre snapshot final",
	"8. executer reconciliation finale",
	"9. basculer DNS/edge",
	"11. lancer smoke tests",
	"13. valider go/no-go final",
];

const requiredFinalValidationMappings = [
	"post-migration-audit.md approuve par Migration lead, Security lead et owners produit",
	"tous les secrets legacy inutiles sont revoques",
	"tous les jobs legacy sont arretes ou supprimes",
	"tous les runbooks publics et prives sont a jour",
	"aucun backlog V2 ne contient une dette necessaire au bon fonctionnement V1",
];

const normalizedContent = normalizeText(content);
const normalizedCompletionRequirements = normalizeText(completionRequirements);

function normalizeText(value) {
	return value.replace(/`/g, "").replace(/\s+/g, " ").trim();
}

if (lineCount > 499) errors.push(`${path}: ${lineCount} lines exceeds 499 line cap`);

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const artifact of requiredArtifacts) {
	if (!content.includes(artifact)) errors.push(`missing required artifact reference: ${artifact}`);
}

for (const command of requiredCommands) {
	if (!content.includes(command)) errors.push(`missing validation command: ${command}`);
}

for (const requirement of cutoverRequirements) {
	const command = normalizeText(strictCommandFor(requirement));
	if (!normalizedContent.includes(command)) errors.push(`missing cutover strict command: ${requirement.id}`);
}

for (const script of migrationScripts) {
	if (!content.includes(`pnpm ${script}`)) {
		errors.push(`missing package migration command: pnpm ${script}`);
	}
}

for (const step of requiredCutoverSteps) {
	if (!content.includes(step)) errors.push(`missing cutover step: ${step}`);
}

for (const requirement of requiredFinalValidationMappings) {
	if (!normalizedContent.includes(requirement)) {
		errors.push(`missing final validation requirement: ${requirement}`);
	}
	if (!normalizedCompletionRequirements.includes(requirement)) {
		errors.push(`${completionRequirementsPath}: missing completion audit mapping for ${requirement}`);
	}
}

for (const [index, line] of lines.entries()) {
	const trimmed = line.trim();
	const previous = lines[index - 1]?.trim();
	if (trimmed && trimmed === previous && !trimmed.startsWith("|")) {
		errors.push(`duplicate consecutive line ${index + 1}: ${trimmed}`);
	}
}

if (errors.length > 0) {
	console.error("Runbook checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Runbook: ok (${requiredArtifacts.length} artifacts, ${migrationScripts.length} migration commands)`,
);
