import { coreArtifacts } from "./check-artifacts.catalog.core.mjs";
import { evidenceArtifacts } from "./check-artifacts.catalog.evidence.mjs";
import { operationsArtifacts } from "./check-artifacts.catalog.operations.mjs";

export const required = [
	...coreArtifacts,
	...evidenceArtifacts,
	...operationsArtifacts,
];
