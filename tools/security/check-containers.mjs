#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";

const errors = [];

function requirePath(path) {
	if (!existsSync(path)) errors.push(`${path}: missing`);
}

function requireText(path, text) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return;
	}
	if (!readFileSync(path, "utf8").includes(text))
		errors.push(`${path}: missing ${text}`);
}

requireText("deploy/oss/helm/nvbes/Chart.yaml", "apiVersion: v2");
requireText("deploy/oss/helm/nvbes/values.yaml", "imageRegistry:");
requireText("deploy/oss/compose/compose.yaml", "services:");
requireText("deploy/oss/kustomize/kustomization.yaml", "resources:");
requirePath("deploy/oss/opentofu/README.md");
requireText(
	"deploy/security/kyverno/verify-nvbes-images.yaml",
	"require-keyless-release-signature",
);
requireText(
	"deploy/security/kyverno/require-restricted-containers.yaml",
	"readOnlyRootFilesystem: true",
);
requireText(".github/workflows/container-release.yml", "cosign sign --yes");
requireText(
	".github/workflows/container-release.yml",
	"actions/attest-build-provenance@",
);
requireText(".github/workflows/container-release.yml", "format: cyclonedx");
requireText(".dockerignore", ".env");
requireText(".dockerignore", "target");

const dockerfiles = [
	"apps/account-worker/Dockerfile",
	"apps/billing-worker/Dockerfile",
	"apps/cloud-worker/Dockerfile",
];

for (const dockerfile of dockerfiles) {
	requireText(dockerfile, "FROM rust:1.85-slim-bookworm@sha256:");
	requireText(dockerfile, "FROM debian:bookworm-slim@sha256:");
	requireText(dockerfile, "cargo build --locked --release");
	requireText(dockerfile, "USER 10001:10001");
}

if (errors.length > 0) {
	console.error("Container/deploy manifest checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log("Container/deploy manifests: ok");
