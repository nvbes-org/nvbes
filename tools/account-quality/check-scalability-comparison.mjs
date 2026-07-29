import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import {
	rejectUnverifiableControlPlaneEvidence,
	validateScalabilityComparisonSchema,
	validateScalabilityMetadata,
} from "../load-tests/account/scalability-evidence-policy.js";
import { validateTargetOrigin } from "../load-tests/account/target-policy.js";

const comparisonPath =
	process.argv[2] || process.env.ACCOUNT_SCALABILITY_COMPARISON_FILE;
if (!comparisonPath) {
	throw new Error("ACCOUNT_SCALABILITY_COMPARISON_FILE is required.");
}

const absoluteComparisonPath = path.resolve(comparisonPath);
const comparisonRoot = path.dirname(absoluteComparisonPath);
const verdictPath =
	process.env.ACCOUNT_SCALABILITY_VERDICT_FILE ||
	path.join(comparisonRoot, "scalability-verdict.json");
const comparison = JSON.parse(await readFile(absoluteComparisonPath, "utf8"));

validateScalabilityComparisonSchema(comparison);

const evaluatedRuns = [];
for (const run of comparison.runs) {
	const metadata = await readJson(run.metadata, "metadata");
	const summary = await readJson(run.summary, "summary");
	const replicas = metadata.replicas;
	const throughput = metric(summary, "iterations", "rate");
	const dropped = metric(summary, "dropped_iterations", "count");
	const errorRate = metric(summary, "http_req_failed", "value");
	const checkRate = metric(summary, "checks", "value");
	const p95 = metric(summary, "http_req_duration", "p(95)");
	const p99 = metric(summary, "http_req_duration", "p(99)");

	validateScalabilityMetadata(metadata, path.basename(run.summary));
	assert(
		summary.setup_data == null,
		"scalability evidence must not contain setup_data",
	);
	validateTargetOrigin({
		allowedOrigins: process.env.ACCOUNT_LOAD_ALLOWED_ORIGINS,
		kind: "service",
		productionOrigins: process.env.ACCOUNT_PRODUCTION_DENIED_ORIGINS,
		target: metadata.targetOrigin,
		targetEnvironment: metadata.targetEnvironment,
	});
	assert(dropped === 0, `replica ${replicas} run dropped iterations`);
	assert(
		throughput >= metadata.configuredLoadRate * 0.98,
		`replica ${replicas} run did not achieve 98% of configured throughput`,
	);
	assert(
		errorRate < 0.005,
		`replica ${replicas} run exceeded the error-rate SLO`,
	);
	assert(
		checkRate > 0.995,
		`replica ${replicas} run exceeded the check-failure SLO`,
	);
	assert(p95 < 400, `replica ${replicas} run exceeded the p95 SLO`);
	assert(p99 < 800, `replica ${replicas} run exceeded the p99 SLO`);
	evaluatedRuns.push({ metadata, replicas, throughput });
}

evaluatedRuns.sort((left, right) => left.replicas - right.replicas);
assert(
	evaluatedRuns.map(({ replicas }) => replicas).join(",") === "1,2,4,8",
	"scalability replica set must be exactly 1, 2, 4 and 8",
);

const baseline = evaluatedRuns[0];
const sharedKeys = [
	"configuredDuration",
	"datasetId",
	"release",
	"suite",
	"targetEnvironment",
	"targetOrigin",
];
for (const run of evaluatedRuns.slice(1)) {
	for (const key of sharedKeys) {
		assert(
			run.metadata[key] === baseline.metadata[key],
			`scalability ${key} must be identical`,
		);
	}
	const efficiency = run.throughput / (baseline.throughput * run.replicas);
	assert(efficiency >= 0.7, `replica ${run.replicas} efficiency is below 70%`);
}

assertVerdictPathSafe(verdictPath);
await writeFile(
	verdictPath,
	`${JSON.stringify(
		{
			datasetId: baseline.metadata.datasetId,
			generatedAt: new Date().toISOString(),
			reason: "control-plane-attestation-verifier-unavailable",
			release: baseline.metadata.release,
			reportedRuns: evaluatedRuns.map(({ metadata, replicas, throughput }) => ({
				achievedThroughput: throughput,
				configuredLoadRate: metadata.configuredLoadRate,
				reportedReplicas: replicas,
			})),
			schemaVersion: 1,
			verdict: "blocked",
		},
		null,
		2,
	)}\n`,
	{ encoding: "utf8", mode: 0o600 },
);
rejectUnverifiableControlPlaneEvidence();

function assertVerdictPathSafe(candidatePath) {
	const resolvedVerdictPath = path.resolve(candidatePath);
	const relativeVerdictPath = path.relative(
		comparisonRoot,
		resolvedVerdictPath,
	);
	assert(
		relativeVerdictPath !== ".." &&
			!relativeVerdictPath.startsWith(`..${path.sep}`) &&
			!path.isAbsolute(relativeVerdictPath),
		"scalability verdict path must stay inside the evidence directory",
	);

	const inputPaths = [
		absoluteComparisonPath,
		...comparison.runs.flatMap((run) => [
			path.resolve(comparisonRoot, run.metadata),
			path.resolve(comparisonRoot, run.summary),
		]),
	];
	assert(
		!inputPaths.includes(resolvedVerdictPath),
		"scalability verdict path must not overwrite an input artifact",
	);
}

async function readJson(relativePath, label) {
	assert(
		typeof relativePath === "string" && relativePath !== "",
		`${label} path is required`,
	);
	const resolvedPath = path.resolve(comparisonRoot, relativePath);
	const relativeResolvedPath = path.relative(comparisonRoot, resolvedPath);
	assert(
		relativeResolvedPath !== ".." &&
			!relativeResolvedPath.startsWith(`..${path.sep}`) &&
			!path.isAbsolute(relativeResolvedPath),
		`${label} path must stay inside the evidence directory`,
	);
	return JSON.parse(await readFile(resolvedPath, "utf8"));
}

function metric(summary, metricName, valueName) {
	const metricValues = summary.metrics?.[metricName];
	const value = metricValues?.values?.[valueName] ?? metricValues?.[valueName];
	assert(
		typeof value === "number" && Number.isFinite(value),
		`${metricName}.${valueName} missing`,
	);
	return value;
}

function assert(condition, message) {
	if (!condition) {
		throw new Error(message);
	}
}
