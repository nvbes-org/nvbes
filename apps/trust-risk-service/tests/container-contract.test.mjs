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
const metricsSource = readFileSync(
	join(serviceRoot, "src/trust_risk.metrics.rs"),
	"utf8",
);
const deploymentWorkflow = readFileSync(
	join(workspaceRoot, ".github/workflows/deploy-trust-risk.yml"),
	"utf8",
);
const runtimeTerraform = readFileSync(
	join(
		workspaceRoot,
		"infrastructure/environments/trust-risk-production/runtime.tf",
	),
	"utf8",
);
const observabilityTerraform = readFileSync(
	join(
		workspaceRoot,
		"infrastructure/environments/trust-risk-production/observability.tf",
	),
	"utf8",
);

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

test("deployment materializes the data-only database identity before runtime validation", () => {
	const foundationStart = deploymentWorkflow.indexOf(
		"- name: Plan migration foundation",
	);
	const validationStart = deploymentWorkflow.indexOf(
		"- name: Validate production runtime configuration",
	);
	const foundation = deploymentWorkflow.slice(foundationStart, validationStart);

	assert.ok(foundationStart >= 0);
	assert.ok(validationStart > foundationStart);
	assert.ok(
		foundation.includes(
			"-target=scaleway_iam_api_key.trust_risk_database_runtime",
		),
	);
	assert.ok(
		foundation.includes(
			"-target=scaleway_iam_policy.trust_risk_database_runtime",
		),
	);
});

test("new image builds use the local Docker runner for linux/amd64", () => {
	assert.ok(
		deploymentWorkflow.includes(
			"build-scan-sign:\n    needs: [ci-test-gate]\n    runs-on: [self-hosted, macOS, ARM64]",
		),
	);
	assert.ok(deploymentWorkflow.includes("platforms: linux/amd64"));
	assert.equal(deploymentWorkflow.includes("ubuntu-24.04"), false);
});

test("deployment can reuse only a signed artifact with unchanged runtime inputs", () => {
	assert.ok(
		deploymentWorkflow.includes("TRUST_RISK_REUSE_SOURCE_IMAGE_DIGEST"),
	);
	assert.ok(
		deploymentWorkflow.includes("TRUST_RISK_REUSE_SOURCE_COMMIT_SHA"),
	);
	assert.ok(
		deploymentWorkflow.includes(
			'git diff --quiet "$SOURCE_COMMIT_SHA" "$GITHUB_SHA" --',
		),
	);
});

test("artifact reuse keeps the CI gate lightweight after source equivalence", () => {
	assert.ok(
		deploymentWorkflow.includes("- name: Verify reused Trust/Risk source"),
	);
	assert.ok(
		deploymentWorkflow.includes(
			"if: vars.TRUST_RISK_REUSE_SOURCE_IMAGE_DIGEST == ''",
		),
	);
});

test("runtime initializes Sentry and authenticated Grafana OTLP", () => {
	assert.ok(mainSource.includes("init_error_reporting_with_config"));
	assert.ok(mainSource.includes("protocol: nvbes_observability::OtlpProtocol::Http"));
	assert.ok(metricsSource.includes(".with_http()"));
	assert.ok(mainSource.includes("otlp_endpoint: config.otlp_endpoint.as_deref()"));
	assert.ok(
		mainSource.includes(
			"otlp_authorization_header: config.otlp_authorization_header.as_deref()",
		),
	);
	assert.ok(runtimeTerraform.includes("SENTRY_RELEASE"));
	assert.ok(runtimeTerraform.includes("SENTRY_TRACES_SAMPLE_RATE"));
	assert.ok(runtimeTerraform.includes("NVBES_OTLP_ENDPOINT"));
	assert.ok(runtimeTerraform.includes("NVBES_OTLP_AUTHORIZATION_HEADER"));
});

test("error reporting smoke receives the complete production runtime config", () => {
	const smokeCommand = '"$SOURCE_TRUST_RISK_IMAGE_DIGEST" error-reporting-smoke';
	const smokeEnd = deploymentWorkflow.indexOf(smokeCommand);
	const smokeStart = deploymentWorkflow.lastIndexOf(
		"docker run --rm --platform linux/amd64",
		smokeEnd,
	);
	const smokeInvocation = deploymentWorkflow.slice(smokeStart, smokeEnd);

	assert.ok(smokeStart >= 0);
	assert.ok(smokeEnd > smokeStart);
	for (const variable of [
		"NVBES_TRUST_RISK_BIND_ADDR",
		"NVBES_TRUST_RISK_SIGNALS_RETENTION_DAYS",
		"NVBES_TRUST_RISK_EVALUATIONS_RETENTION_DAYS",
		"NVBES_TRUST_RISK_LABELS_RETENTION_DAYS",
		"NVBES_TRUST_RISK_REVIEWS_RETENTION_DAYS",
		"NVBES_TRUST_RISK_AUDIT_RETENTION_DAYS",
	]) {
		assert.ok(smokeInvocation.includes(`--env ${variable}`), variable);
	}
});

test("deployment provisions Trust/Risk Grafana and injects observability secrets", () => {
	assert.ok(observabilityTerraform.includes('resource "grafana_folder"'));
	assert.ok(observabilityTerraform.includes('resource "grafana_dashboard"'));
	assert.ok(observabilityTerraform.includes('resource "grafana_rule_group"'));
	assert.equal(observabilityTerraform.includes("org_id"), false);
	assert.ok(deploymentWorkflow.includes("TRUST_RISK_SENTRY_DSN"));
	assert.ok(deploymentWorkflow.includes("GRAFANA_SERVICE_ACCOUNT_TOKEN"));
	assert.ok(deploymentWorkflow.includes("GRAFANA_OTLP_AUTHORIZATION_HEADER"));
});
