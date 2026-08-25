import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const workspaceRoot = resolve(
	dirname(fileURLToPath(import.meta.url)),
	"../../..",
);
const serviceRoot = join(workspaceRoot, "apps/trust-risk-service");
const dockerfile = readFileSync(join(serviceRoot, "Dockerfile"), "utf8");
const manifest = readFileSync(join(serviceRoot, "Cargo.toml"), "utf8");
const mainSource = readFileSync(join(serviceRoot, "src/main.rs"), "utf8");

test("image builds and runs the Trust/Risk service as non-root", () => {
	assert.match(manifest, /^name = "nvbes-trust-risk-service"$/m);
	assert.match(
		dockerfile,
		/^FROM rust:1\.91\.1-slim-bookworm@sha256:[a-f0-9]{64} AS builder$/m,
	);
	assert.match(
		dockerfile,
		/cargo build --locked --release --bin nvbes-trust-risk-service/,
	);
	assert.equal(
		dockerfile.match(/^ARG DEBIAN_FRONTEND=noninteractive$/gm)?.length,
		2,
	);
	assert.ok(dockerfile.includes("CARGO_BUILD_JOBS=1"));
	assert.ok(dockerfile.includes("CMAKE_BUILD_PARALLEL_LEVEL=1"));
	assert.ok(dockerfile.includes("USER 10001:10001"));
	assert.ok(dockerfile.includes("EXPOSE 8080"));
	assert.ok(dockerfile.includes('NVBES_TRUST_RISK_BIND_ADDR="0.0.0.0:8080"'));
	assert.ok(dockerfile.includes('ENTRYPOINT ["/app/trust-risk-service"]'));
});

test("container exposes shallow liveness and supports deployment commands", () => {
	assert.match(
		dockerfile,
		/HEALTHCHECK[^\n]*\\\n\s+CMD \["curl", "--fail", "--silent", "--show-error", "http:\/\/127\.0\.0\.1:8080\/health\/live"\]/,
	);
	assert.ok(dockerfile.includes("STOPSIGNAL SIGTERM"));
	assert.ok(mainSource.includes('action == "migrate"'));
	assert.ok(mainSource.includes('action == "validate-runtime"'));
	assert.ok(mainSource.includes("SignalKind::terminate()"));
});
