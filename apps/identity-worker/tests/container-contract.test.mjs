import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, posix, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
	compareVersions,
	lockedPackageGraphSnapshot,
	offlineMetadataMsrv,
} from "../../../tools/container-contract/cargo-lock.mjs";

const workspaceRoot = resolve(
	dirname(fileURLToPath(import.meta.url)),
	"../../..",
);
const workerRoot = join(workspaceRoot, "apps/identity-worker");
const manifestPath = join(workerRoot, "Cargo.toml");
const dockerfilePath = join(workerRoot, "Dockerfile");
const mainPath = join(workerRoot, "src/main.rs");

const manifest = readFileSync(manifestPath, "utf8");
const workspaceManifest = readFileSync(
	join(workspaceRoot, "Cargo.toml"),
	"utf8",
);
const lockfile = readFileSync(join(workspaceRoot, "Cargo.lock"), "utf8");
const dockerfile = readFileSync(dockerfilePath, "utf8");
const mainSource = readFileSync(mainPath, "utf8");

const lockResolution = {
	msrv: "1.91.1",
	packageGraphSha256:
		"5a1373d06ebd7e8cbe2c90bb13918c3e738ef6862224ed3ee876acfe7ae3d3c2",
};

const expectedCliModes = [
	"error-reporting-smoke",
	"migrate",
	"run-audit-anchor",
	"run-housekeeping",
];

function tomlSection(source, name) {
	const lines = source.split(/\r?\n/);
	const result = [];
	let active = false;

	for (const line of lines) {
		if (/^\s*\[/.test(line)) {
			active = line.trim() === `[${name}]`;
			continue;
		}
		if (active) result.push(line);
	}

	return result.join("\n");
}

function tomlString(section, key) {
	return section.match(new RegExp(`^${key}\\s*=\\s*"([^"]+)"`, "m"))?.[1];
}

function packageContract() {
	const packageSection = tomlSection(manifest, "package");
	const packageName = tomlString(packageSection, "name");
	assert.ok(packageName, "Cargo.toml must declare [package].name");

	const explicitBinName = manifest.match(
		/^\[\[bin\]\][\s\S]*?^name\s*=\s*"([^"]+)"/m,
	)?.[1];
	const binaryName = explicitBinName ?? packageName;

	let edition = tomlString(packageSection, "edition");
	if (!edition && /^edition\.workspace\s*=\s*true\s*$/m.test(packageSection)) {
		edition = tomlString(
			tomlSection(workspaceManifest, "workspace.package"),
			"edition",
		);
	}
	assert.ok(edition, "the package edition must resolve from Cargo.toml");

	return { binaryName, edition, packageName };
}

function dockerInstructions(source) {
	const instructions = [];
	let current = "";

	for (const rawLine of source.split(/\r?\n/)) {
		const line = rawLine.trim();
		if (!current && (!line || line.startsWith("#"))) continue;
		current += `${current ? " " : ""}${line.replace(/\\$/, "").trim()}`;
		if (!line.endsWith("\\")) {
			instructions.push(current);
			current = "";
		}
	}

	assert.equal(
		current,
		"",
		"Dockerfile must not end with an unfinished instruction",
	);
	return instructions;
}

function dockerJsonInstruction(name) {
	const instruction = dockerInstructions(dockerfile).find((line) =>
		line.startsWith(`${name} `),
	);
	assert.ok(instruction, `Dockerfile must declare ${name}`);
	return JSON.parse(instruction.slice(name.length).trim());
}

function argumentValue(tokens, flag) {
	const position = tokens.indexOf(flag);
	if (position >= 0) return tokens[position + 1];
	return tokens
		.find((token) => token.startsWith(`${flag}=`))
		?.slice(flag.length + 1);
}

function dockerBuildContract() {
	const instructions = dockerInstructions(dockerfile);
	const builder = instructions
		.find((line) => /^FROM rust:/.test(line))
		?.match(
			/^FROM rust:(?<version>\d+\.\d+(?:\.\d+)?)-slim-bookworm@sha256:(?<digest>[a-f0-9]{64}) AS builder$/,
		);
	assert.ok(
		builder?.groups,
		"builder image must pin rust:<version>-slim-bookworm by sha256",
	);

	const build = instructions.find((line) => /^RUN cargo build\b/.test(line));
	assert.ok(build, "Dockerfile must contain a cargo build instruction");
	const buildTokens = build.split(/\s+/).slice(1);
	assert.ok(
		buildTokens.includes("--locked"),
		"container build must honor Cargo.lock",
	);
	assert.ok(
		buildTokens.includes("--release"),
		"container build must produce a release binary",
	);

	const binaryName = argumentValue(buildTokens, "--bin");
	const packageName = argumentValue(buildTokens, "--package");
	assert.ok(binaryName, "container build must select an explicit --bin");
	assert.ok(packageName, "container build must select an explicit --package");

	const copy = instructions.find(
		(line) =>
			/^COPY --from=builder\b/.test(line) && line.includes("/target/release/"),
	);
	assert.ok(copy, "Dockerfile must copy the release binary from the builder");
	const copyPaths = copy
		.split(/\s+/)
		.slice(1)
		.filter((token) => !token.startsWith("--"));
	assert.equal(
		copyPaths.length,
		2,
		"binary COPY must have one source and one destination",
	);

	const [copySource, copyDestination] = copyPaths;
	const entrypoint = dockerJsonInstruction("ENTRYPOINT");
	assert.deepEqual(
		entrypoint,
		[copyDestination],
		"ENTRYPOINT must execute the copied binary",
	);

	return {
		binaryName,
		builderVersion: builder.groups.version,
		copyDestination,
		copySource,
		packageName,
	};
}

test("Cargo build, Docker copy and entrypoint select the same binary", () => {
	const cargo = packageContract();
	const container = dockerBuildContract();

	assert.equal(container.packageName, cargo.packageName);
	assert.equal(container.binaryName, cargo.binaryName);
	assert.equal(posix.basename(container.copySource), cargo.binaryName);
	assert.equal(posix.basename(container.copyDestination), cargo.binaryName);
});

test("builder toolchain satisfies the locked dependency MSRV", (context) => {
	const cargo = packageContract();
	const container = dockerBuildContract();
	const editionMsrv = { 2021: "1.56.0", 2024: "1.85.0" }[cargo.edition];
	assert.ok(
		editionMsrv,
		`unsupported Rust edition in contract: ${cargo.edition}`,
	);

	const graphHash = lockedPackageGraphSnapshot(lockfile, cargo.packageName);
	assert.equal(
		graphHash,
		lockResolution.packageGraphSha256,
		`locked ${cargo.packageName} graph changed; resolve and review its MSRV (now ${graphHash})`,
	);

	const metadata = offlineMetadataMsrv(workspaceRoot, cargo.packageName);
	if (metadata.error) {
		context.diagnostic(
			`offline cargo metadata unavailable; using reviewed lock snapshot`,
		);
	} else {
		assert.equal(
			metadata.msrv,
			lockResolution.msrv,
			"reviewed lock MSRV is stale",
		);
	}

	const required =
		compareVersions(lockResolution.msrv, editionMsrv) >= 0
			? lockResolution.msrv
			: editionMsrv;
	assert.ok(
		compareVersions(container.builderVersion, required) >= 0,
		`Rust ${container.builderVersion} is below resolved MSRV ${required}`,
	);
});

test("container default and every supported CLI mode remain explicit", () => {
	const sourceModes = [
		...mainSource.matchAll(
			/matches!\(arg1\.as_deref\(\), Some\("([^"]+)"\)\)/g,
		),
	]
		.map((match) => match[1])
		.sort();
	assert.deepEqual(sourceModes, expectedCliModes);
	assert.match(mainSource, /worker::run_loop_until_shutdown\(/);

	const command = dockerJsonInstruction("CMD");
	assert.deepEqual(command, ["run-housekeeping"]);
	assert.ok(
		sourceModes.includes(command[0]),
		"container CMD must be a supported CLI mode",
	);
});
