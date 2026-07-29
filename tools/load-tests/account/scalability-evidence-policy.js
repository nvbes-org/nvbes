export const CONTROL_PLANE_GAP_MESSAGE =
	"Scalability evidence is blocked: no trusted control-plane attestation verifier is configured.";

const COMPARISON_KEYS = ["runs", "schemaVersion"];
const RUN_KEYS = ["metadata", "summary"];
const METADATA_KEYS = [
	"configuredDuration",
	"configuredIterations",
	"configuredLoadRate",
	"datasetId",
	"generatedAt",
	"profile",
	"release",
	"replicas",
	"schemaVersion",
	"suite",
	"summaryFile",
	"targetEnvironment",
	"targetOrigin",
];

export function validateScalabilityComparisonSchema(comparison) {
	assertPlainRecord(comparison, "scalability comparison");
	assertExactKeys(comparison, COMPARISON_KEYS, "scalability comparison");
	assert(comparison.schemaVersion === 1, "scalability schemaVersion must be 1");
	assert(Array.isArray(comparison.runs), "scalability runs must be an array");
	assert(
		comparison.runs.length === 4,
		"scalability requires exactly 1, 2, 4 and 8 replicas",
	);

	for (const run of comparison.runs) {
		assertPlainRecord(run, "scalability run");
		assertExactKeys(run, RUN_KEYS, "scalability run");
		assertPath(run.metadata, "metadata");
		assertPath(run.summary, "summary");
		assert(
			run.metadata !== run.summary,
			"metadata and summary paths must be distinct",
		);
	}
}

export function validateScalabilityMetadata(metadata, expectedSummaryFile) {
	assertPlainRecord(metadata, "scalability metadata");
	assertExactKeys(metadata, METADATA_KEYS, "scalability metadata");
	assert(
		metadata.schemaVersion === 1,
		"every metadata file must use schemaVersion 1",
	);
	assert(
		metadata.profile === "scalability",
		"every run must use the scalability profile",
	);
	assert(
		["account-api", "account-session"].includes(metadata.suite),
		"every run must use an Account load suite",
	);
	assert(
		metadata.targetEnvironment === "staging",
		"scalability evidence must come from staging",
	);
	assert(
		metadata.configuredDuration === "15m",
		"scalability configuredDuration must be exactly 15m",
	);
	assert(
		metadata.configuredIterations === null,
		"scalability configuredIterations must be null",
	);
	assertPositiveInteger(metadata.configuredLoadRate, "configuredLoadRate");
	assertPositiveInteger(metadata.replicas, "replicas");
	assert(
		typeof metadata.datasetId === "string" &&
			/^[a-z0-9][a-z0-9._-]{0,79}$/iu.test(metadata.datasetId),
		"every run must record a valid synthetic dataset identifier",
	);
	assert(
		typeof metadata.release === "string" &&
			/^[a-f0-9]{7,64}$/u.test(metadata.release),
		"scalability release must be an immutable 7-64 character hexadecimal revision",
	);
	assert(
		typeof metadata.generatedAt === "string" &&
			Number.isFinite(Date.parse(metadata.generatedAt)) &&
			new Date(metadata.generatedAt).toISOString() === metadata.generatedAt,
		"scalability generatedAt must be a canonical UTC timestamp",
	);
	assert(
		typeof metadata.targetOrigin === "string" && metadata.targetOrigin !== "",
		"scalability targetOrigin is required",
	);
	assert(
		typeof metadata.summaryFile === "string" &&
			/^[a-zA-Z0-9][a-zA-Z0-9._-]*\.json$/u.test(metadata.summaryFile),
		"scalability summaryFile must be a safe JSON filename",
	);
	assert(
		metadata.summaryFile === expectedSummaryFile,
		"metadata and summary pair mismatch",
	);
}

export function rejectUnverifiableControlPlaneEvidence() {
	throw new Error(CONTROL_PLANE_GAP_MESSAGE);
}

function assertExactKeys(value, expected, label) {
	const actual = Object.keys(value).sort();
	const sortedExpected = [...expected].sort();
	assert(
		actual.length === sortedExpected.length &&
			actual.every((key, index) => key === sortedExpected[index]),
		`${label} must contain exactly: ${sortedExpected.join(", ")}`,
	);
}

function assertPath(value, label) {
	assert(
		typeof value === "string" && value !== "" && value === value.trim(),
		`${label} path is required`,
	);
}

function assertPlainRecord(value, label) {
	assert(
		typeof value === "object" &&
			value !== null &&
			!Array.isArray(value) &&
			Object.getPrototypeOf(value) === Object.prototype,
		`${label} must be a JSON object`,
	);
}

function assertPositiveInteger(value, label) {
	assert(
		Number.isSafeInteger(value) && value > 0,
		`${label} must be a positive integer`,
	);
}

function assert(condition, message) {
	if (!condition) {
		throw new Error(message);
	}
}
