import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const serviceRoot = join(workspaceRoot, "apps/identity-service");
const dockerfile = readFileSync(join(serviceRoot, "Dockerfile"), "utf8");
const mainSource = readFileSync(join(serviceRoot, "src/main.rs"), "utf8");
const healthSource = readFileSync(
	join(serviceRoot, "src/identity.health.rs"),
	"utf8",
);

test("image is reproducible and runs Identity as non-root", () => {
	assert.match(
		dockerfile,
		/^FROM rust:1\.91\.1-slim-bookworm@sha256:[a-f0-9]{64} AS builder$/m,
	);
	assert.match(
		dockerfile,
		/cargo build --locked --release --bin nvbes-identity-service/,
	);
	assert.equal(
		dockerfile.match(/^ARG DEBIAN_FRONTEND=noninteractive$/gm)?.length,
		2,
	);
	assert.ok(dockerfile.includes("CARGO_BUILD_JOBS=1"));
	assert.ok(dockerfile.includes("USER 10001:10001"));
	assert.ok(dockerfile.includes("EXPOSE 8080"));
	assert.ok(dockerfile.includes('ENTRYPOINT ["/app/identity-service"]'));
});

test("container has shallow liveness and graceful shutdown", () => {
	assert.match(
		dockerfile,
		/HEALTHCHECK[^\n]*\\\n\s+CMD \["curl", "--fail", "--silent", "--show-error", "http:\/\/127\.0\.0\.1:8080\/health\/live"\]/,
	);
	assert.ok(dockerfile.includes("STOPSIGNAL SIGTERM"));
	assert.ok(mainSource.includes('action == "migrate"'));
	assert.ok(mainSource.includes("SignalKind::terminate()"));
	assert.ok(healthSource.includes('sqlx::query_scalar::<_, i32>("SELECT 1")'));
});

test("runtime stays closed to public authentication", () => {
	assert.equal(mainSource.includes('route("/auth/register"'), false);
	assert.equal(mainSource.includes('route("/auth/login"'), false);
	assert.ok(mainSource.includes("init_error_reporting_with_config"));
	assert.ok(mainSource.includes("otlp_authorization_header"));
});
