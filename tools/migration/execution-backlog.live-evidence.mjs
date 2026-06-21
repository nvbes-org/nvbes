import { buildPreparationCommands } from "./live-evidence-commands.mjs";

export function preparationCommandsForRequirement(requirement) {
	if (!requirement?.id || !Array.isArray(requirement.expected_evidence)) return [];
	return buildPreparationCommands([requirement]).filter((command) =>
		command.includes(`--packet-requirement ${requirement.id}`),
	);
}
