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
	assert.equal(existsSync(`${stackRoot}/email-observability.tf`), true);
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
	assert.doesNotMatch(workflow, /^\s+inputs:\s*$/mu);
	assert.match(workflow, /name:\s*production-email/u);
	assert.match(workflow, /refs\/heads\/main/u);
	assert.match(
		workflow,
		/EMAIL_DEPLOY_CONFIRMATION: \$\{\{ vars\.EMAIL_DEPLOY_CONFIRMATION \}\}/u,
	);
	assert.match(
		workflow,
		/EMAIL_DEPLOY_APPROVED_SHA: \$\{\{ vars\.EMAIL_DEPLOY_APPROVED_SHA \}\}/u,
	);
	assert.match(
		workflow,
		/\[\[ "\$EMAIL_DEPLOY_CONFIRMATION" == "deploy-email-production" \]\]/u,
	);
	assert.match(
		workflow,
		/\[\[ "\$EMAIL_DEPLOY_APPROVED_SHA" =~ \^\[0-9a-f\]\{40\}\$ \]\]/u,
	);
	assert.match(
		workflow,
		/\[\[ "\$EMAIL_DEPLOY_APPROVED_SHA" == "\$GITHUB_SHA" \]\]/u,
	);
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
	assert.match(
		workflow,
		/https:\/\/api\.scaleway\.com\/account\/v3\/projects\/\$SCW_DEFAULT_PROJECT_ID/u,
	);
	assert.match(workflow, /X-Auth-Token: \$SCW_SECRET_KEY/u);
	assert.match(workflow, /SCW_DEFAULT_ORGANIZATION_ID=%s/u);
	assert.match(
		workflow,
		/\^\[0-9a-f\]\{8\}-\[0-9a-f\]\{4\}-\[1-5\]\[0-9a-f\]\{3\}-\[89ab\]\[0-9a-f\]\{3\}-\[0-9a-f\]\{12\}\$/u,
	);
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
		/name: Plan email runtime foundation[\s\S]*?-target=scaleway_container\.email_runtime[\s\S]*?name: Validate production runtime and Sentry reporting configuration[\s\S]*?show -json email-runtime-foundation\.tfplan[\s\S]*?email_database_runtime_url\.value[\s\S]*?"\$SOURCE_EMAIL_IMAGE_DIGEST" validate-runtime[\s\S]*?name: Apply reviewed email runtime foundation plan[\s\S]*?email-runtime-foundation\.tfplan[\s\S]*?name: Plan isolated email runtime/u,
	);
	assert.match(
		workflow,
		/TF_VAR_email_observability_internal_token: \$\{\{ secrets\.EMAIL_OBSERVABILITY_INTERNAL_TOKEN \}\}/u,
	);
	assert.match(
		workflow,
		/NVBES_OBSERVABILITY_INTERNAL_TOKEN: \$\{\{ secrets\.EMAIL_OBSERVABILITY_INTERNAL_TOKEN \}\}/u,
	);
	assert.match(
		workflow,
		/TF_VAR_email_sentry_dsn: \$\{\{ secrets\.EMAIL_SENTRY_DSN \}\}/u,
	);
	assert.match(
		workflow,
		/TF_VAR_email_database_runtime_credential_generation: \$\{\{ vars\.EMAIL_DATABASE_RUNTIME_CREDENTIAL_GENERATION \}\}/u,
	);
	assert.match(
		workflow,
		/TF_VAR_email_internal_validation_enabled: \$\{\{ vars\.EMAIL_SYNTHETIC_SMOKE_ENABLED \}\}/u,
	);
	assert.match(
		workflow,
		/\[\[ "\$EMAIL_SYNTHETIC_SMOKE_ENABLED" == "true" \]\]/u,
	);
	assert.match(
		workflow,
		/TF_VAR_grafana_service_account_token: \$\{\{ secrets\.GRAFANA_SERVICE_ACCOUNT_TOKEN \}\}/u,
	);
	assert.match(workflow, /error-reporting-smoke/u);
	assert.match(workflow, /\.status == "sent"[\s\S]*?\.flushed == true/u);
	assert.match(workflow, /email_worker_metrics_endpoint/u);
	assert.match(workflow, /Authorization: Bearer \$EMAIL_OBSERVABILITY_INTERNAL_TOKEN/u);
	assert.match(workflow, /anonymous_status[\s\S]*?== "401"/u);
	assert.match(
		workflow,
		/if: \$\{\{ vars\.EMAIL_SYNTHETIC_SMOKE_ENABLED == 'true' \}\}/u,
	);
	assert.match(
		workflow,
		/NVBES_EMAIL_GRPC_AUTH_TOKEN: \$\{\{ secrets\.EMAIL_SYNTHETIC_PRODUCER_TOKEN \}\}/u,
	);
	assert.match(
		workflow,
		/NVBES_EMAIL_SYNTHETIC_RECIPIENT: \$\{\{ secrets\.EMAIL_SYNTHETIC_RECIPIENT \}\}/u,
	);
	assert.match(workflow, /synthetic-smoke/u);
	assert.match(
		workflow,
		/\.delivery_state == "delivered" and \.processed_provider_events > 0/u,
	);
	assert.match(workflow, /output -raw email_database_runtime_url/u);
	assert.match(workflow, /--env NVBES_EMAIL_DATABASE_URL/u);
	assert.match(
		workflow,
		/name: 'Post-deploy: Close the bounded Email validation window'[\s\S]*?if: \$\{\{ always\(\) \}\}[\s\S]*?TF_VAR_email_internal_validation_enabled=false/u,
	);
	assert.match(
		workflow,
		/-target=scaleway_container\.email_runtime[\s\S]*?-target=scaleway_container_trigger\.email_dispatch[\s\S]*?-target=scaleway_container_trigger\.email_retention/u,
	);
	assert.match(workflow, /email-economic-shutdown\.tfplan/u);
	assert.match(workflow, /ingress_privacy[\s\S]*?== "private"/u);
	assert.match(workflow, /trigger_count[\s\S]*?== "0"/u);
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
	const provider = read(`${stackRoot}/email-provider.tf`);
	const outputs = read(`${stackRoot}/outputs.tf`);
	const observability = read(`${stackRoot}/email-observability.tf`);
	assert.match(
		delivery,
		/resource "scaleway_registry_namespace" "email_worker"/u,
	);
	assert.match(delivery, /is_public\s*=\s*false/u);
	assert.match(delivery, /resource "scaleway_container" "email_runtime"/u);
	assert.match(
		delivery,
		/privacy\s*=\s*var\.email_internal_validation_enabled \? "public" : "private"/u,
	);
	assert.match(
		delivery,
		/resource "scaleway_container_trigger" "email_dispatch" \{[\s\S]*?count\s*=\s*var\.email_internal_validation_enabled \? 1 : 0/u,
	);
	assert.match(
		delivery,
		/resource "scaleway_container_trigger" "email_retention" \{[\s\S]*?count\s*=\s*var\.email_internal_validation_enabled \? 1 : 0/u,
	);
	assert.match(
		delivery,
		/startup_probe\s*\{[\s\S]*?interval\s*=\s*"(?:[5-9]|[1-9]\d+)s"[\s\S]*?\}/u,
	);
	assert.match(
		delivery,
		/NVBES_OBSERVABILITY_INTERNAL_TOKEN\s*=\s*var\.email_observability_internal_token/u,
	);
	assert.match(delivery, /SENTRY_DSN\s*=\s*var\.email_sentry_dsn/u);
	assert.match(delivery, /SENTRY_RELEASE\s*=\s*split/u);
	assert.match(observability, /resource "grafana_dashboard" "email_communications"/u);
	assert.match(observability, /resource "grafana_rule_group" "email"/u);
	assert.match(observability, /nvbes-email-production-v1/u);
	assert.doesNotMatch(observability, /^\s*org_id\s*=/mu);
	const reusableObservability = read(
		"infrastructure/stacks/email/production/email-observability.tf",
	);
	assert.match(reusableObservability, /nvbes-email-production-v1/u);
	assert.doesNotMatch(reusableObservability, /^\s*org_id\s*=/mu);
	assert.match(
		delivery,
		/private_network_id\s*=\s*[^\n]*var\.private_network_id/u,
	);
	assert.match(database, /resource "scaleway_sdb_sql_database" "email"/u);
	assert.match(
		database,
		/resource "terraform_data" "email_database_runtime_credential_generation"/u,
	);
	assert.match(
		database,
		/resource "scaleway_iam_api_key" "email_database_runtime"[\s\S]*?create_before_destroy\s*=\s*true[\s\S]*?replace_triggered_by\s*=\s*\[terraform_data\.email_database_runtime_credential_generation\]/u,
	);
	assert.match(
		database,
		/split\([\s\S]*?"\?"[\s\S]*?trimprefix\(scaleway_sdb_sql_database\.email\.endpoint, "postgres:\/\/"\)[\s\S]*?\)\[0\]/u,
	);
	assert.match(
		database,
		/"postgres:\/\/%s:%s@%s\?sslmode=verify-full"/u,
	);
	assert.match(
		provider,
		/resource "scaleway_mnq_sns_credentials" "email_events_terraform"[\s\S]*?can_receive\s*=\s*true/u,
	);
	assert.match(
		outputs,
		/output "email_database_runtime_url"[\s\S]*?sensitive\s*=\s*true/u,
	);
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
