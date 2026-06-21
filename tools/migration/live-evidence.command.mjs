import { normalize } from "node:path";

export function commandMatchesStrictCommand(command, strictCommand) {
	if (typeof command !== "string" || typeof strictCommand !== "string") return false;
	const commandValue = normalizeCommand(command);
	return strictCommand.split(" && ").some((segment) => commandMatchesSegment(commandValue, normalizeCommand(segment)));
}

export function sourceArtifactMatchesReportArgument(instance) {
	const reportPath = commandOptionValue(instance.command, "--report");
	if (!reportPath) return true;
	return normalize(instance.source_artifact ?? "") === normalize(reportPath);
}

export function environmentMatchesCommandArgument(instance) {
	const env = commandOptionValue(instance.command, "--env");
	if (!env) return true;
	return instance.environment === env;
}

export function commandReportPath(command) {
	return commandOptionValue(command, "--report");
}

function commandMatchesSegment(command, segment) {
	const pattern = `^${segment.split("<run>").map(escapeRegExp).join("[a-zA-Z0-9][a-zA-Z0-9._-]*")}$`;
	return new RegExp(pattern).test(command);
}

function normalizeCommand(command) {
	return command.trim().replace(/\s+/g, " ");
}

function escapeRegExp(value) {
	return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function commandOptionValue(command, option) {
	const tokens = tokenizeCommand(command);
	const optionIndex = tokens.indexOf(option);
	if (optionIndex >= 0) return tokens[optionIndex + 1];
	const optionPrefix = `${option}=`;
	const inlineOption = tokens.find((token) => token.startsWith(optionPrefix));
	return inlineOption ? inlineOption.slice(optionPrefix.length) : undefined;
}

function tokenizeCommand(command) {
	return typeof command === "string" ? command.trim().split(/\s+/).filter(Boolean) : [];
}
