import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const workerRoot = join(workspaceRoot, "apps/email-worker");
const dockerfile = readFileSync(join(workerRoot, "Dockerfile"), "utf8");
const manifest = readFileSync(join(workerRoot, "Cargo.toml"), "utf8");
const mainSource = readFileSync(join(workerRoot, "src/main.rs"), "utf8");

test("image builds and runs the Rust email worker", () => {
	assert.match(manifest, /^name = "nvbes-email-worker"$/m);
	assert.match(
		dockerfile,
		/^FROM rust:1\.91\.1-slim-bookworm@sha256:[a-f0-9]{64} AS builder$/m,
	);
	assert.match(
		dockerfile,
		/RUN cargo build --locked --release --bin nvbes-email-worker/,
	);
	assert.ok(dockerfile.includes("COPY contracts ./contracts"));
	assert.ok(dockerfile.includes("USER 10001:10001"));
	assert.ok(dockerfile.includes("EXPOSE 3040"));
	assert.ok(!dockerfile.includes("EXPOSE 3040 3041"));
	assert.ok(dockerfile.includes('NVBES_EMAIL_HTTP_BIND_ADDR="0.0.0.0:3040"'));
	assert.ok(dockerfile.includes('NVBES_EMAIL_GRPC_BIND_ADDR="0.0.0.0:3040"'));
	assert.ok(dockerfile.includes('ENTRYPOINT ["/app/email-worker"]'));
	assert.ok(mainSource.includes("tonic::service::Routes::from(http_router)"));
});

test("container liveness uses the shallow HTTP endpoint", () => {
	assert.match(
		dockerfile,
		/HEALTHCHECK[^\n]*\\\n\s+CMD \["curl", "--fail", "--silent", "--show-error", "http:\/\/127\.0\.0\.1:3040\/health\/live"\]/,
	);
	assert.ok(dockerfile.includes("STOPSIGNAL SIGTERM"));
	assert.ok(!dockerfile.includes('"/health/ready"'));
	assert.ok(mainSource.includes("SignalKind::terminate()"));
});

test("Terraform bootstrap exposes liveness before production secrets exist", () => {
	const bootstrapGuard = mainSource.indexOf('action == "deployment-bootstrap"');
	const productionConfig = mainSource.indexOf("EmailWorkerConfig::from_env()");

	assert.ok(bootstrapGuard >= 0);
	assert.ok(productionConfig > bootstrapGuard);
	assert.ok(mainSource.includes('"/health/live"'));
	assert.ok(mainSource.includes("StatusCode::NO_CONTENT"));
});
