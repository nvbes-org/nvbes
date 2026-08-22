import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const workflowPath = ".github/workflows/deploy-email.yml";
const stackRoot = "infrastructure/environments/email-production";

function read(path) {
	return readFileSync(path, "utf8");
}

test("email production deploy is isolated and uses an immutable signed image", () => {
	assert.equal(existsSync(workflowPath), true);
	assert.equal(existsSync(`${stackRoot}/backend.tf`), true);
	assert.equal(existsSync(`${stackRoot}/email-database.tf`), true);
	assert.equal(existsSync(`${stackRoot}/email-delivery.tf`), true);
	assert.equal(
		existsSync("infrastructure/environments/production/email-database.tf"),
		false,
	);
	assert.equal(
		existsSync("infrastructure/environments/production/email-delivery.tf"),
		false,
	);
	const handoff = read(
		"infrastructure/environments/production/email-state-handoff.tf",
	);
	assert.match(handoff, /from = scaleway_tem_domain\.transactional/u);
	assert.match(handoff, /from = scaleway_sdb_sql_database\.email/u);
	assert.match(handoff, /from = scaleway_container\.email_runtime/u);
	assert.match(handoff, /destroy = false/u);

	const workflow = read(workflowPath);
	assert.match(workflow, /^\s*workflow_dispatch:\s*$/mu);
	assert.match(workflow, /name:\s*production-email/u);
	assert.match(workflow, /refs\/heads\/main/u);
	assert.match(
		workflow,
		/nvbes-org\/nvbes-email-worker:\$\{\{ github\.sha \}\}/u,
	);
	assert.doesNotMatch(
		workflow,
		/^\s*\$\{\{ env\.REGISTRY \}\}\/\$\{\{ env\.IMAGE_NAME \}\}:\$\{\{ github\.ref_name \}\}\s*$/mu,
	);
	assert.match(workflow, /cosign verify/u);
	assert.match(workflow, /docker login ghcr\.io/u);
	assert.match(workflow, /-target=scaleway_registry_namespace\.email_worker/u);
	assert.match(workflow, /docker login "\$EMAIL_REGISTRY_ENDPOINT"/u);
	assert.match(workflow, /docker pull --platform linux\/amd64/u);
	assert.match(workflow, /docker push "\$destination_tag"/u);
	assert.match(workflow, /cosign sign --yes "\$EMAIL_IMAGE_DIGEST"/u);
	assert.match(workflow, /production\/email\/terraform\.tfstate/u);
	assert.match(
		workflow,
		/-target=scaleway_job_definition\.email_database_migration/u,
	);
	assert.match(workflow, /\/job-definitions\/\$\{job_definition_id\}\/start/u);
	assert.match(workflow, /migration_state.*succeeded/su);
	assert.match(
		workflow,
		/terraform[\s\S]*?plan[\s\S]*?-out=email-runtime\.tfplan/u,
	);
	assert.match(
		workflow,
		/terraform[\s\S]*?apply[\s\S]*?email-runtime\.tfplan/u,
	);
	assert.match(workflow, /\/health\/live/u);
	assert.doesNotMatch(workflow, /terraform[\s\S]*?apply[\s\S]*?-auto-approve/u);

	const variables = read(`${stackRoot}/variables.tf`);
	assert.match(variables, /\^rg\\\\\.\[a-z\]\{2\}/u);
	assert.match(variables, /nvbes-email-worker@sha256:\[0-9a-f\]\{64\}/u);
	assert.doesNotMatch(variables, /default\s*=\s*"[^\n]*email-worker/u);

	const delivery = read(`${stackRoot}/email-delivery.tf`);
	const database = read(`${stackRoot}/email-database.tf`);
	assert.match(
		delivery,
		/resource "scaleway_registry_namespace" "email_worker"/u,
	);
	assert.match(delivery, /is_public\s*=\s*false/u);
	assert.match(delivery, /resource "scaleway_container" "email_runtime"/u);
	assert.match(
		delivery,
		/private_network_id\s*=\s*[^\n]*var\.private_network_id/u,
	);
	assert.match(database, /resource "scaleway_sdb_sql_database" "email"/u);
});

test("container release publishes the image name consumed by Terraform", () => {
	const deployables = JSON.parse(read("tools/ci/deployable-applications.json"));
	const email = deployables.rust.find(
		(application) => application.project === "email-worker",
	);
	assert.equal(email.image, "nvbes-email-worker");
});

test("email CI validates the isolated deployment stack", () => {
	const workflow = read(workflowPath);
	const packageScripts = JSON.parse(read("package.json")).scripts;
	assert.match(
		workflow,
		/  ci-test-gate:\n    runs-on: [^\n]+\n    timeout-minutes:\s*(?:4[5-9]|[5-9]\d|\d{3,})/u,
	);
	assert.doesNotMatch(workflow, /Swatinem\/rust-cache@/u);
	assert.doesNotMatch(workflow, /^\s+cache: pnpm\s*$/mu);
	assert.match(workflow, /pnpm test:pre-deploy/u);
	assert.match(workflow, /pnpm nx run email-worker:test/u);
	assert.doesNotMatch(workflow, /pnpm test:unit/u);
	assert.match(
		workflow,
		/name: Generate CycloneDX SBOM[\s\S]*?format: cyclonedx(?:\s|$)/u,
	);
	assert.doesNotMatch(workflow, /format: cyclonedx-json/u);
	assert.match(packageScripts["test:pre-deploy"], /terraform fmt -check/u);
	assert.match(
		workflow,
		/terraform -chdir=infrastructure\/environments\/email-production init[\s\S]*?-backend=false/u,
	);
	assert.match(
		workflow,
		/terraform -chdir=infrastructure\/environments\/email-production validate/u,
	);
	assert.match(
		workflow,
		/terraform -chdir=infrastructure\/stacks\/email\/production init[\s\S]*?-backend=false/u,
	);
	assert.match(
		workflow,
		/terraform -chdir=infrastructure\/stacks\/email\/production validate/u,
	);
});
