#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/drive-upload-download.generated.json";
const markdownPath = "docs/migration/drive-upload-download.md";

const sources = {
	uploadLogic: "apps/cloud-service/src/drive.domains.uploads.logic.rs",
	uploadCore: "apps/cloud-service/src/drive.domains.uploads.core.rs",
	uploadComplete: "apps/cloud-service/src/drive.domains.uploads.lifecycle.complete.rs",
	uploadTusAppend: "apps/cloud-service/src/drive.domains.uploads.lifecycle.tus.append.rs",
	uploadTusFinalize: "apps/cloud-service/src/drive.domains.uploads.lifecycle.tus.finalize.rs",
	publicUploads: "apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.uploads.rs",
	downloadRoutes: "apps/cloud-service/src/drive.domains.files.routes.download.rs",
	downloadStream: "apps/cloud-service/src/drive.domains.files.transfer.stream.rs",
	downloadUrl: "apps/cloud-service/src/drive.domains.files.transfer.download_url.rs",
	downloadRange: "apps/cloud-service/src/drive.domains.files.transfer.range.rs",
	publicDownload: "apps/cloud-service/src/drive.domains.public_api.routes.v1_handlers.files.download.rs",
	workerStorage: "apps/cloud-worker/src/drive.workers.maintenance.storage.rs",
	workerDispatch: "apps/cloud-worker/src/drive.workers.executor.dispatch.rs",
	openapiSource: "apps/cloud-service/src/drive.http.openapi.rs",
	openapiJson: "apps/cloud-service/openapi.json",
	dataMap: "docs/migration/data-map.md",
	jobMap: "docs/migration/job-map.generated.json",
};

const errors = [];
const packageScripts = readPackageScripts(errors);

function read(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}

function textCheck(id, path, description, pattern) {
	const content = read(path);
	return {
		id,
		description,
		path,
		status: content.includes(pattern) ? "passed" : "failed",
		pattern,
	};
}

function buildChecks() {
	return [
		textCheck("upload-name-validation", sources.uploadLogic, "Upload logic validates object names", "validate_object_name"),
		textCheck("upload-size-cap-test", sources.uploadLogic, "Upload size cap is covered by unit test", "validate_size_rejects_values_above_processing_cap"),
		textCheck("upload-encoding-test", sources.uploadLogic, "Upload content encoding is covered by unit tests", "ensure_plain_content_encoding_rejects_gzip"),
		textCheck("upload-object-key", sources.uploadLogic, "Upload object keys are workspace/object/upload scoped", "workspaces/{workspace_id}/objects/{storage_object_id}/{upload_id}"),
		textCheck("upload-signed-url", sources.uploadLogic, "Upload logic signs object-storage upload URLs", "presign_upload"),
		textCheck("upload-create-quota", sources.uploadCore, "Upload creation checks quota before accepting metadata", "ensure_upload_allowed_tx"),
		textCheck("upload-create-pending-object", sources.uploadCore, "Upload creation inserts pending storage object", "insert_pending_storage_object_tx"),
		textCheck("upload-create-session", sources.uploadCore, "Upload creation inserts upload session", "insert_upload_session_tx"),
		textCheck("upload-create-audit", sources.uploadCore, "Upload creation is audited", "\"upload.created\""),
		textCheck("tus-multipart-create", sources.uploadCore, "TUS upload creates multipart object-storage upload", "create_multipart_upload"),
		textCheck("public-upload-create", sources.publicUploads, "Public API exposes upload creation", "path = \"/v1/workspaces/{workspaceId}/uploads\""),
		textCheck("public-upload-complete", sources.publicUploads, "Public API exposes upload completion", "path = \"/v1/workspaces/{workspaceId}/uploads/{uploadId}/complete\""),
		textCheck("public-upload-scope", sources.publicUploads, "Public upload API requires files:write scope", "\"files:write\""),
		textCheck("complete-storage-head", sources.uploadComplete, "Upload completion reads object-storage metadata", "storage.head_object(object_key)"),
		textCheck("complete-storage-read", sources.uploadComplete, "Upload completion downloads uploaded object for validation and scan", "storage.get_object(object_key)"),
		textCheck("complete-size-check", sources.uploadComplete, "Upload completion verifies declared and stored size", "upload_size_mismatch"),
		textCheck("complete-checksum", sources.uploadComplete, "Upload completion verifies SHA-256 checksum", "upload_checksum_mismatch"),
		textCheck("complete-scan", sources.uploadComplete, "Upload completion scans uploaded bytes", "scan::perform_scan"),
		textCheck("complete-activate", sources.uploadComplete, "Upload completion activates or quarantines storage object", "activate_storage_object_tx"),
		textCheck("complete-quota", sources.uploadComplete, "Upload completion records quota usage", "record_file_uploaded_tx"),
		textCheck("complete-audit", sources.uploadComplete, "Upload completion is audited as file uploaded or quarantined", "\"file.uploaded\""),
		textCheck("tus-upload-part", sources.uploadTusAppend, "TUS append uploads object-storage parts", "upload_part(object_key"),
		textCheck("tus-complete-multipart", sources.uploadTusAppend, "TUS append completes multipart upload", "complete_multipart_upload"),
		textCheck("tus-finalize-quota", sources.uploadTusFinalize, "TUS finalization records quota usage", "record_file_uploaded_tx"),
		textCheck("download-route", sources.downloadRoutes, "Drive download stream route is exposed", "/workspaces/{workspaceId}/objects/{objectId}/download"),
		textCheck("download-url-route", sources.downloadRoutes, "Drive download URL route is exposed", "/workspaces/{workspaceId}/objects/{objectId}/download-url"),
		textCheck("download-authz", sources.downloadRoutes, "Download routes require DownloadFile authorization", "WorkspaceAction::DownloadFile"),
		textCheck("download-stream-full", sources.downloadStream, "Download stream reads full object from object storage", "storage.get_object(&object_key)"),
		textCheck("download-stream-range", sources.downloadStream, "Download stream supports object-storage range reads", "get_object_range"),
		textCheck("download-bandwidth", sources.downloadStream, "Download stream records bandwidth usage", "record_bandwidth_out_tx"),
		textCheck("download-audit", sources.downloadStream, "Download stream is audited", "\"file.downloaded\""),
		textCheck("download-url-presign", sources.downloadUrl, "Download URL flow signs object-storage download URLs", "presign_download"),
		textCheck("download-url-bandwidth", sources.downloadUrl, "Download URL flow records bandwidth usage", "record_bandwidth_out_tx"),
		textCheck("public-download-url", sources.publicDownload, "Public API exposes download URL creation", "path = \"/v1/workspaces/{workspaceId}/objects/{objectId}/download-url\""),
		textCheck("public-download-scope", sources.publicDownload, "Public download API requires files:read scope", "\"files:read\""),
		textCheck("downloadability-test", sources.downloadRange, "Downloadability rules are covered by tests", "ensure_downloadable_file_accepts_active_files_only"),
		textCheck("range-test", sources.downloadRange, "Download range behavior is covered by tests", "resolve_range_supports_full_suffix_and_clamped_ranges"),
		textCheck("range-reject-test", sources.downloadRange, "Invalid ranges are covered by tests", "resolve_range_rejects_multiple_ranges_and_marks_unsatisfiable"),
		textCheck("storage-purge-deleted", sources.workerStorage, "Worker purges deleted object-storage keys and metadata", "purge_deleted_storage"),
		textCheck("storage-purge-quarantined", sources.workerStorage, "Worker purges quarantined object-storage keys and metadata", "purge_quarantined"),
		textCheck("storage-delete-objects", sources.workerStorage, "Worker deletes object-storage keys before metadata cleanup", "storage.delete_objects(keys)"),
		textCheck("worker-dispatch-storage", sources.workerDispatch, "Worker dispatch includes storage purge jobs", "JOB_STORAGE_PURGE_DELETED"),
		textCheck("openapi-source", sources.openapiSource, "Drive OpenAPI source includes public upload/download routes", "crate::domains::public_api::v1_handlers::create_upload"),
		textCheck("openapi-upload-json", sources.openapiJson, "Generated OpenAPI includes public upload path", "\"/v1/workspaces/{workspaceId}/uploads\""),
		textCheck("openapi-download-json", sources.openapiJson, "Generated OpenAPI includes public download URL path", "\"/v1/workspaces/{workspaceId}/objects/{objectId}/download-url\""),
		textCheck("data-map-storage", sources.dataMap, "Storage objects require object/link reconciliation", "object_or_link_invariant"),
		textCheck("data-map-upload-sessions", sources.dataMap, "Upload sessions are explicitly mapped for migration", "`target-postgres:drive.upload_sessions`"),
		textCheck("job-map-storage-reconciliation", sources.jobMap, "Storage jobs require storage reconciliation evidence", "storage_reconciliation"),
	];
}

function summarize(checks) {
	const failed = checks.filter((check) => check.status === "failed").length;
	return {
		checks: checks.length,
		passed: checks.length - failed,
		failed,
		status: failed === 0 ? "passed" : "failed",
	};
}

function sameItems(actual, expected) {
	return Array.isArray(actual) && actual.length === expected.length && actual.every((item, index) => item === expected[index]);
}

function validateReport(report) {
	const seen = new Set();
	for (const check of report.checks) {
		if (seen.has(check.id)) errors.push(`${outputPath}: duplicate check ${check.id}`);
		seen.add(check.id);
		if (!check.description) errors.push(`${check.id}: description is required`);
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in drive upload/download source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/drive-upload-download.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match drive upload/download source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-cloud-service upload --locked",
		"cargo test -p nvbes-cloud-service download --locked",
		"cargo test -p nvbes-cloud-service resolve_range --locked",
		"pnpm check:migration-reconciliation-report",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "drive-upload-download", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Drive Upload Download Evidence",
		"",
		"## Status",
		"",
		`- status: ${data.summary.status}`,
		`- checks: ${data.summary.checks}`,
		`- passed: ${data.summary.passed}`,
		`- failed: ${data.summary.failed}`,
		"",
		"## Rules",
		"",
		"- Every evidence row must be generated from the drive upload/download source contract.",
		"- `passed` requires the configured file or generated migration document to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name upload, download, range, and reconciliation checks required by parity.",
		"- Generation provenance must identify sources, write command and targeted tests.",
		"",
		"## Evidence",
		"",
		"| Check | Status | Path |",
		"|---|---:|---|",
	];
	for (const check of data.checks) {
		lines.push(`| ${check.description} | ${check.status} | \`${check.path}\` |`);
	}
	lines.push(
		"",
		"## Decision",
		"",
		data.summary.failed === 0
			? "Drive upload/download parity evidence is covered for repository cutover gates. Production cutover still requires accepted object-storage reconciliation counts."
			: "Drive upload/download parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-drive-upload-download",
		"node tools/migration/drive-upload-download.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

const checks = buildChecks();
const summary = summarize(checks);
const report = {
	schema_version: 1,
	generation: {
		command: "node tools/migration/drive-upload-download.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-cloud-service upload --locked",
			"cargo test -p nvbes-cloud-service download --locked",
			"cargo test -p nvbes-cloud-service resolve_range --locked",
			"pnpm check:migration-reconciliation-report",
		],
	},
	summary,
	checks,
};

validateReport(report);

const json = serializeJson(report);
const markdown = serializeMarkdown(report);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Drive upload/download evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/drive-upload-download.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/drive-upload-download.mjs --write`);
}

if (errors.length > 0) {
	console.error("Drive upload/download evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Drive upload/download evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
