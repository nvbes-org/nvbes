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

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const serviceRoot = join(workspaceRoot, "apps/identity-service");
const manifest = readFileSync(join(serviceRoot, "Cargo.toml"), "utf8");
const workspaceManifest = readFileSync(join(workspaceRoot, "Cargo.toml"), "utf8");
const lockfile = readFileSync(join(workspaceRoot, "Cargo.lock"), "utf8");
const dockerfile = readFileSync(join(serviceRoot, "Dockerfile"), "utf8");
const mainSource = readFileSync(join(serviceRoot, "src/main.rs"), "utf8");

const lockResolution = {
	msrv: "1.98.1",
	packageGraphSha256:
		"0e33b26020a240600577921e195717c16987ea4e2873072ec0a99d6a7a88c8f9",
};

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
	const binaryName = manifest.match(
		/^\[\[bin\]\][\s\S]*?^name\s*=\s*"([^"]+)"/m,
	)?.[1];
	let edition = tomlString(packageSection, "edition");
	if (!edition && /^edition\.workspace\s*=\s*true\s*$/m.test(packageSection)) {
		edition = tomlString(tomlSection(workspaceManifest, "workspace.package"), "edition");
	}
	assert.ok(packageName && binaryName && edition, "Cargo package contract must be explicit");
	return { binaryName, edition, packageName };
}

function dockerInstructions() {
	const instructions = [];
	let current = "";
	for (const rawLine of dockerfile.split(/\r?\n/)) {
		const line = rawLine.trim();
		if (!current && (!line || line.startsWith("#"))) continue;
		current += `${current ? " " : ""}${line.replace(/\\$/, "").trim()}`;
		if (!line.endsWith("\\")) {
			instructions.push(current);
			current = "";
		}
	}
	assert.equal(current, "", "Dockerfile must not end with an unfinished instruction");
	return instructions;
}

function jsonInstruction(instructions, name) {
	const instruction = instructions.find((line) => line.startsWith(`${name} `));
	assert.ok(instruction, `Dockerfile must declare ${name}`);
	return JSON.parse(instruction.slice(name.length).trim());
}

function argumentValue(tokens, flag) {
	const position = tokens.indexOf(flag);
	return position >= 0 ? tokens[position + 1] : undefined;
}

test("build, copy and entrypoint select the Identity Service binary", () => {
	const cargo = packageContract();
	assert.match(
		manifest,
		/^utoipa-swagger-ui\s*=\s*\{[^\n]*features\s*=\s*\["vendored"\][^\n]*\}$/m,
		"Swagger UI must be vendored instead of downloaded during the image build",
	);
	const instructions = dockerInstructions();
	assert.ok(
		instructions.includes("COPY contracts ./contracts"),
		"builder must include the protobuf contracts consumed by build.rs",
	);
	const builder = instructions
		.find((line) => /^FROM rust:/.test(line))
		?.match(/^FROM rust:(?<version>\d+\.\d+(?:\.\d+)?)-slim-bookworm@sha256:[a-f0-9]{64} AS builder$/);
	assert.ok(builder?.groups, "builder image must pin Rust by sha256");
	const build = instructions.find((line) => /^RUN cargo build\b/.test(line));
	assert.ok(build, "Dockerfile must build the service");
	const tokens = build.split(/\s+/).slice(1);
	assert.ok(tokens.includes("--locked") && tokens.includes("--release"));
	assert.equal(argumentValue(tokens, "--package"), cargo.packageName);
	assert.equal(argumentValue(tokens, "--bin"), cargo.binaryName);
	const copy = instructions.find(
		(line) => /^COPY --from=builder\b/.test(line) && line.includes("/target/release/"),
	);
	assert.ok(copy, "runtime must copy the release binary");
	const paths = copy.split(/\s+/).slice(1).filter((token) => !token.startsWith("--"));
	assert.equal(posix.basename(paths[0]), cargo.binaryName);
	assert.deepEqual(jsonInstruction(instructions, "ENTRYPOINT"), [paths[1]]);
	assert.ok(compareVersions(builder.groups.version, lockResolution.msrv) >= 0);
});

test("reviewed dependency graph and MSRV remain current", (context) => {
	const cargo = packageContract();
	assert.equal(
		lockedPackageGraphSnapshot(lockfile, cargo.packageName),
		lockResolution.packageGraphSha256,
		"locked Identity Service graph changed; review the container MSRV",
	);
	const metadata = offlineMetadataMsrv(workspaceRoot, cargo.packageName);
	if (metadata.error) context.diagnostic(metadata.error);
	else assert.equal(metadata.msrv, lockResolution.msrv);
});

test("runtime is non-root, observable and exposes only explicit service ports", () => {
	const instructions = dockerInstructions();
	assert.ok(dockerfile.includes("deb http://snapshot.debian.org/"));
	assert.ok(!dockerfile.includes("Acquire::AllowInsecureRepositories"));
	assert.ok(dockerfile.includes("      libclang-dev \\"));
	assert.ok(dockerfile.includes("      make \\"));
	assert.ok(dockerfile.includes("      perl \\"));
	assert.ok(instructions.includes("USER 10001:10001"));
	assert.ok(instructions.includes("EXPOSE 4000 4010"));
	assert.ok(instructions.includes("STOPSIGNAL SIGTERM"));
	assert.ok(
		instructions.some(
			(line) =>
				line.startsWith("HEALTHCHECK ") &&
				line.includes("http://127.0.0.1:4000/health"),
		),
		"runtime health check must target the public health endpoint",
	);
	assert.ok(dockerfile.includes('NVBES_API_PORT="4000"'));
	assert.ok(dockerfile.includes('NVBES_IDENTITY_GRPC_PORT="4010"'));
	assert.ok(mainSource.includes("SignalKind::terminate()"));
});
