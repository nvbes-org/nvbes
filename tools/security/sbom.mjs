#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { dirname } from "node:path";

const args = process.argv.slice(2);
const outputArgument = args.find((argument) =>
	argument.startsWith("--output="),
);
const outputPath =
	outputArgument?.slice("--output=".length) ?? ".temp/security/nvbes.cdx.json";

function fail(message) {
	console.error(`SBOM generation failed: ${message}`);
	process.exit(1);
}

const version = spawnSync("trivy", ["--version"], { encoding: "utf8" });
if (version.error?.code === "ENOENT") {
	fail(
		"trivy is required; install the pinned CI version before running pnpm check:sbom",
	);
}
if (version.status !== 0) {
	fail(version.stderr.trim() || "unable to execute trivy");
}

mkdirSync(dirname(outputPath), { recursive: true });
const result = spawnSync(
	"trivy",
	[
		"filesystem",
		"--format",
		"cyclonedx",
		"--output",
		outputPath,
		"--skip-dirs",
		"node_modules",
		"--skip-dirs",
		"target",
		"--skip-dirs",
		".nx",
		"--skip-dirs",
		".temp",
		"--no-progress",
		".",
	],
	{ encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
);

if (result.status !== 0) {
	fail(
		result.stderr.trim() ||
			result.stdout.trim() ||
			"trivy returned a non-zero status",
	);
}
if (!existsSync(outputPath)) {
	fail(`${outputPath} was not created`);
}

let sbom;
try {
	sbom = JSON.parse(readFileSync(outputPath, "utf8"));
} catch (error) {
	fail(`${outputPath} is not valid JSON: ${error.message}`);
}

if (sbom.bomFormat !== "CycloneDX")
	fail(`${outputPath}: bomFormat must be CycloneDX`);
if (typeof sbom.specVersion !== "string")
	fail(`${outputPath}: specVersion is missing`);
if (!Array.isArray(sbom.components) || sbom.components.length === 0) {
	fail(`${outputPath}: no dependency components were discovered`);
}
if (!sbom.metadata?.timestamp)
	fail(`${outputPath}: metadata.timestamp is missing`);

console.log(
	`CycloneDX SBOM: ok (${sbom.components.length} components, spec ${sbom.specVersion}, ${outputPath})`,
);
