#!/usr/bin/env node
import { readFileSync } from "node:fs";

const path = "docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md";
const content = readFileSync(path, "utf8");
const lines = content.split("\n");
const completionRequirementsPath = "tools/migration/completion-requirements.mjs";
const completionRequirements = readFileSync(completionRequirementsPath, "utf8");

const requiredHeadings = [
	"# Plan Structuration Complete Big Bang Zero Dette",
	"## Objectif",
	"## Principe Central",
	"## Stack Cible",
	"## Structure Monorepo Cible",
	"## Frontieres de Domaine",
	"## Contrats",
	"## Data Architecture",
	"## Frontend",
	"## OSS, Cloud et Internal",
	"## Phases de Reconstruction",
	"## Gates de Decision",
	"## Backlog Executable",
	"## Definition de Done Domaine",
	"## Boundaries de Code",
	"## Risques Bloquants",
	"## Criteres de Reussite",
];

const requiredPhases = [
	"1. Freeze et inventaire",
	"2. Nouvelle fondation",
	"3. Primitives plateforme",
	"4. Identity",
	"5. Workspace/Authz",
	"6. Drive",
	"7. Billing/Usage",
	"8. Developer Platform",
	"9. Frontends",
	"10. Infra/deploy",
	"11. Repetitions migration",
	"12. Big Bang cutover",
	"13. Decommission",
];

const requiredGates = [
	"G0 Freeze",
	"G1 Fondation",
	"G2 Primitives",
	"G3 Domaines",
	"G4 Frontends",
	"G5 Infra",
	"G6 Repetitions",
	"G7 Cutover",
	"G8 Decommission",
];

const requiredChecks = [
	"pnpm check",
	"pnpm check:migration-precutover",
	"node tools/migration/gate-evidence.mjs --strict",
];

const requiredSuccessCriteria = [
	"tous les parcours critiques",
	"toutes les donnees migrables",
	"aucun composant runtime legacy",
	"aucun provider proprietaire",
	"le runbook de cutover",
	"chaque gate a une preuve",
];

const requiredCompletionMappings = [
	"tous les parcours critiques ont une implementation nouvelle",
	"tous les contrats publics et internes sont versionnes",
	"toutes les donnees migrables sont reconciliees",
	"aucun composant runtime legacy n'est requis",
	"aucun provider proprietaire n'est dans le core OSS",
	"aucun import interdit OSS/Cloud/Internal ne passe la CI",
	"les backups et rollbacks sont testes",
	"le repo public OSS ne contient aucun document prive",
	"le runbook de cutover peut etre execute sans decision implicite",
	"chaque gate a une preuve attachee",
	"chaque risque bloquant a ete ferme, accepte par owner, ou retire du scope",
];

const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const phase of requiredPhases) {
	if (!content.includes(phase)) errors.push(`missing reconstruction phase: ${phase}`);
}

for (const gate of requiredGates) {
	if (!content.includes(`| ${gate} |`)) errors.push(`missing decision gate: ${gate}`);
}

for (const check of requiredChecks) {
	if (!content.includes(check)) errors.push(`missing verification reference: ${check}`);
}

for (const criterion of requiredSuccessCriteria) {
	if (!content.includes(criterion)) errors.push(`missing success criterion: ${criterion}`);
}

for (const criterion of requiredCompletionMappings) {
	if (!content.includes(criterion)) errors.push(`missing mapped success criterion: ${criterion}`);
	if (!completionRequirements.includes(criterion)) {
		errors.push(`${completionRequirementsPath}: missing completion audit mapping for ${criterion}`);
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
	console.error("Blueprint checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Blueprint: ok (${requiredPhases.length} phases, ${requiredGates.length} gates)`);
