import { commandMatchesStrictCommand } from "./live-evidence.command.mjs";

const commandByType = {
	cutover: "pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json",
	decommission: "pnpm check:migration-postcutover",
	frontend_signoff: "pnpm check:migration-frontend-experience",
	web_check: "pnpm check:web",
	smoke_test: "pnpm check:migration-smoke-tests -- --strict",
	infra_deploy: "pnpm check:migration-infra-deploy",
	infra_restore: "pnpm check:migration-backup-restore -- --strict",
	reconciliation: "node tools/migration/reconcile.mjs --env <env> --report docs/migration/reconciliation.<run>.json",
	rehearsal: "pnpm check:migration-rehearsals -- --strict",
	rollback: "pnpm check:migration-rollback-report -- --strict",
	reject_review: "pnpm check:migration-rejects -- --strict",
};

const artifactByType = {
	cutover: "artifact://migration/<evidence-id>.source",
	decommission: "artifact://migration/<evidence-id>.source",
	frontend_signoff: "artifact://migration/<evidence-id>.source",
	web_check: "artifact://migration/<evidence-id>.source",
	smoke_test: "artifact://migration/<evidence-id>.source",
	infra_deploy: "artifact://migration/<evidence-id>.source",
	infra_restore: "artifact://migration/<evidence-id>.source",
	reconciliation: "docs/migration/reconciliation.<run>.json",
	rehearsal: "artifact://migration/<evidence-id>.source",
	rollback: "artifact://migration/<evidence-id>.source",
	reject_review: "artifact://migration/<evidence-id>.source",
};

export function buildPreparationCommands(requirements) {
	return requirements.flatMap((requirement) =>
		requirement.expected_evidence.flatMap((expected) => {
			const environments = environmentsFor(requirement, expected);
			return environments.map((environment, index) => commandFor(requirement, expected, environment, index + 1));
		}),
	);
}

export function commandCoverageFor(strictCommand, preparationCommands) {
	const strict_command_segments = strictCommandSegments(strictCommand);
	const evidence_command_segments = preparationCommands.map(preparationCommandValue).filter(Boolean);
	const coverage_commands = evidence_command_segments.map((command) => command.replaceAll("<run>", "run"));
	const uncovered_strict_command_segments = strict_command_segments.filter(
		(segment) => !coverage_commands.some((command) => commandMatchesStrictCommand(command, segment)),
	);
	return { strict_command_segments, evidence_command_segments, uncovered_strict_command_segments };
}

function commandFor(requirement, expected, environment, ordinal) {
	const evidenceId = evidenceIdFor(requirement.id, expected.type, environment, ordinal);
	const sourceArtifact = artifactFor(expected.type, evidenceId);
	const command = commandByType[expected.type].replaceAll("<env>", environment);
	const parts = [
		"node",
		"tools/migration/live-evidence-prepare.mjs",
		"--id",
		evidenceId,
		"--type",
		expected.type,
		"--env",
		environment,
		"--owner",
		"<owner>",
		"--source-artifact",
		sourceArtifact,
		"--command",
		command,
		"--result",
		"passed",
		"--decision",
		"go",
		"--immutable-reference",
		`artifact://migration/${evidenceId}`,
		"--packet-requirement",
		requirement.id,
		"--notes",
		"<owner justification and run context>",
		"--out",
		`docs/migration/live-evidence-instances/${evidenceId}.json`,
	];
	if (sourceArtifact.includes("://")) parts.splice(12, 0, "--source-checksum", "<source-artifact-sha256>");
	return parts.map(shellQuote).join(" ");
}

function environmentsFor(requirement, expected) {
	if (expected.environments) return expected.environments;
	if (requirement.id === "p11-rehearsals" && expected.type === "reconciliation") return ["staging", "production"];
	return Array.from({ length: expected.min }, () => environmentFor(requirement, expected));
}

function environmentFor(requirement, expected) {
	if (requirement.id === "g4-frontend-signoff") return "local";
	if (expected.type === "reconciliation") return "production";
	return "staging";
}

function artifactFor(type, evidenceId) {
	return (artifactByType[type] ?? "artifact://migration/<evidence-id>.source").replaceAll("<evidence-id>", evidenceId);
}

function evidenceIdFor(requirementId, type, environment, ordinal) {
	return `${requirementId}-${type}-${environment}-${ordinal}`.replaceAll("_", "-");
}

function shellQuote(value) {
	if (/^[a-zA-Z0-9_./:=@+-]+$/.test(value)) return value;
	return `'${value.replaceAll("'", "'\"'\"'")}'`;
}

function strictCommandSegments(strictCommand) {
	return typeof strictCommand === "string"
		? strictCommand.split(" && ").map((segment) => segment.trim()).filter(Boolean)
		: [];
}

function preparationCommandValue(command) {
	const tokens = tokenize(command);
	const index = tokens.indexOf("--command");
	return index >= 0 ? tokens[index + 1] : undefined;
}

function tokenize(command) {
	const tokens = [];
	const pattern = /'((?:[^']|'"\''"')*)'|"([^"]*)"|(\S+)/g;
	for (const match of command.matchAll(pattern)) {
		tokens.push((match[1] ?? match[2] ?? match[3]).replaceAll("'\"'\"'", "'"));
	}
	return tokens;
}
