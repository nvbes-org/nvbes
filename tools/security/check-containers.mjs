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
	if (!readFileSync(path, "utf8").includes(text)) errors.push(`${path}: missing ${text}`);
}

requireText("deploy/oss/helm/nvbes/Chart.yaml", "apiVersion: v2");
requireText("deploy/oss/helm/nvbes/values.yaml", "imageRegistry:");
requireText("deploy/oss/compose/compose.yaml", "services:");
requireText("deploy/oss/kustomize/kustomization.yaml", "resources:");
requirePath("deploy/oss/opentofu/README.md");

if (errors.length > 0) {
	console.error("Container/deploy manifest checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log("Container/deploy manifests: ok");
