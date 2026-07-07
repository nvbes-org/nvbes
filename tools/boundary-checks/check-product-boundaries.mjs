#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { checkBillingMigrationBoundaries } from "./check-product-boundaries.billing-migrations.mjs";
import { checkBillingWorkerQueueBoundaries } from "./check-product-boundaries.billing-worker-queues.mjs";
import { checkAccountBillingRuntimeBoundary } from "./check-product-boundaries.account-billing-runtime.mjs";
import { checkAccountWebBillingClientBoundary } from "./check-product-boundaries.account-web-billing.mjs";
import { checkBillingProviderEvidence } from "./check-product-boundaries.billing-providers.mjs";
import { checkGatewayCloudBoundary } from "./check-product-boundaries.gateway-cloud.mjs";
import { checkLegacyRuntimeNames } from "./check-product-boundaries.legacy-runtime-names.mjs";
import { checkRustPackageBoundaries } from "./check-product-boundaries.rust-packages.mjs";

const errors = [];
const textExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".sql"]);
const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);

function normalizePath(value) {
	return value.replaceAll("\\", "/");
}

function shouldScan(path) {
	if (path.split("/").some((part) => skippedDirs.has(part))) return false;
	const dot = path.lastIndexOf(".");
	return dot >= 0 && textExtensions.has(path.slice(dot));
}

function walk(dir, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			walk(path, results);
		} else {
			const relativePath = normalizePath(relative(process.cwd(), path));
			if (shouldScan(relativePath)) results.push(relativePath);
		}
	}
	return results;
}

function checkProductSourceImports() {
	for (const root of ["libs/rust/products", "libs/ts/products"]) {
		for (const file of walk(root)) {
			const content = readFileSync(file, "utf8");
			const forbiddenPatterns = [
				/libs\/rust\/adapters-(?:oss|cloud)\//,
				/adapters[-_](?:oss|cloud)/,
				/libs\/ts\/cloud-ui/,
				/from\s+["'][^"']*cloud-ui["']/,
				/from\s+["'][^"']*apps\//,
			];

			for (const pattern of forbiddenPatterns) {
				if (pattern.test(content)) {
					errors.push(`${file}: product source imports forbidden boundary ${pattern}`);
				}
			}
		}
	}
}

function checkDriveBillingBoundary() {
	const forbiddenDriveBillingFiles = [
		/drive\.domains\.billing\.manage(?:\.|$)/,
		/drive\.domains\.billing\.webhooks(?:\.|$)/,
		/drive\.domains\.billing\.routes\.manage\.rs$/,
		/drive\.domains\.billing\.routes\.webhooks\.rs$/,
		/drive\.domains\.billing\.service\.rs$/,
		/drive\.domains\.billing\.stripe\.rs$/,
		/drive\.domains\.billing\.types\.rs$/,
		/drive\.domains\.billing\.usage_events\.rs$/,
	];

	for (const file of walk("apps/cloud-service/src")) {
		const content = readFileSync(file, "utf8");
		for (const pattern of forbiddenDriveBillingFiles) {
			if (pattern.test(file)) {
				errors.push(`${file}: Cloud Service must not own Billing runtime, PSP, or public Billing route files`);
			}
		}
		if (content.includes("nvbes_billing")) {
			errors.push(`${file}: Cloud Service must not import nvbes_billing; use Billing service boundaries instead`);
		}
		if (/["']\/workspaces\/[^"']*\/billing(?:\/|\{|["'])/.test(content)) {
			errors.push(`${file}: Cloud Service must not expose workspace Billing routes`);
		}
		if (/["']\/billing\/webhooks["']/.test(content)) {
			errors.push(`${file}: Cloud Service must not expose PSP Billing webhook routes`);
		}
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+(?:subscriptions|billing_accounts|billing_adjustments|billing_webhook_events|invoice_estimates|usage_snapshots|stripe_price_mappings|billing_provider_customers|billing_provider_price_mappings)\b/i.test(content)) {
			errors.push(`${file}: Cloud Service runtime must not access Billing-owned SQL tables`);
		}
	}

	for (const file of walk("apps/cloud-worker/src")) {
		const content = readFileSync(file, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+(?:subscriptions|billing_accounts|billing_adjustments|billing_webhook_events|invoice_estimates|usage_snapshots|stripe_price_mappings|billing_provider_customers|billing_provider_price_mappings)\b/i.test(content)) {
			errors.push(`${file}: Cloud worker runtime must not access Billing-owned SQL tables`);
		}
	}

	const driveManifest = "apps/cloud-service/Cargo.toml";
	if (existsSync(driveManifest) && readFileSync(driveManifest, "utf8").includes("nvbes-billing")) {
		errors.push(`${driveManifest}: Cloud Service must not depend on nvbes-billing`);
	}

	const driveOpenapi = "apps/cloud-service/openapi.json";
	if (existsSync(driveOpenapi)) {
		const content = readFileSync(driveOpenapi, "utf8");
		if (/["']\/[^"']*\/billing(?:\/|\{|["'])/.test(content)) {
			errors.push(`${driveOpenapi}: Drive OpenAPI must not expose Billing routes`);
		}
		if (/"name"\s*:\s*"billing"/i.test(content) || /"tags"\s*:\s*\[[^\]]*"billing"/i.test(content)) {
			errors.push(`${driveOpenapi}: Drive OpenAPI must not expose Billing tags`);
		}
	}
}

function checkDriveBillingMigrationBoundary() {
	const allowedProjectionPatterns = [
		/\bbilling_entitlement_snapshots\b/i,
		/\bbilling_locked\b/i,
	];
	const forbiddenMigrationPatterns = [
		/\bbilling_provider\b/i,
		/\bsubscription_status\b/i,
		/\bcustomer_type\b/i,
		/\bbilling_adjustment_type\b/i,
		/\bbilling_webhook_status\b/i,
		/\bsubscriptions\b/i,
		/\bbilling_accounts\b/i,
		/\bbilling_adjustments\b/i,
		/\bbilling_webhook_events\b/i,
		/\binvoice_estimates\b/i,
		/\busage_snapshots\b/i,
		/\bstripe_price_mappings\b/i,
		/\bbilling_provider_customers\b/i,
		/\bbilling_provider_price_mappings\b/i,
		/\bbilling_fraud_assessments\b/i,
	];

	for (const file of walk("apps/cloud-service/migrations")) {
		const content = readFileSync(file, "utf8");
		for (const pattern of forbiddenMigrationPatterns) {
			if (pattern.test(content)) {
				errors.push(`${file}: Drive migrations must not own Billing runtime schema ${pattern}`);
			}
		}
		if (/\bbilling_[a-z0-9_]+\b/i.test(content) && !allowedProjectionPatterns.some((pattern) => pattern.test(content))) {
			errors.push(`${file}: Drive migrations may only keep Billing entitlement projection tables`);
		}
	}
}

function checkInternalAdminBillingBoundary() {
	const commandCenter = "apps/backoffice-service/src/internal_admin.command_center.rs";
	if (existsSync(commandCenter)) {
		const content = readFileSync(commandCenter, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${commandCenter}: command center Billing metrics must come from billing-service/gRPC`);
		}
		if (!content.includes("get_admin_command_center_billing_metrics")) {
			errors.push(`${commandCenter}: command center must use Billing gRPC metrics`);
		}
	}

	const operationsCenter = "apps/backoffice-service/src/internal_admin.operations_center.rs";
	if (existsSync(operationsCenter)) {
		const content = readFileSync(operationsCenter, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${operationsCenter}: operations center Billing snapshot must come from billing-service/gRPC`);
		}
		if (!content.includes("get_admin_operations_center")) {
			errors.push(`${operationsCenter}: operations center must use Billing gRPC snapshot`);
		}
	}

	const operationsMutations = "apps/backoffice-service/src/internal_admin.operations_center.mutations.rs";
	if (existsSync(operationsMutations)) {
		const content = readFileSync(operationsMutations, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${operationsMutations}: operations center Billing mutations must call billing-service/gRPC`);
		}
		if (!content.includes("run_admin_operations_action")) {
			errors.push(`${operationsMutations}: operations center Billing mutations must use Billing gRPC actions`);
		}
	}

	for (const file of [
		"apps/backoffice-service/src/internal_admin.billing_platform_center.mutations.rs",
		"apps/backoffice-service/src/internal_admin.billing_platform_center.routing_mutations.rs",
		"apps/backoffice-service/src/internal_admin.billing_fraud_review.actions.rs",
	]) {
		if (!existsSync(file)) continue;
		const content = readFileSync(file, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${file}: Billing Platform mutations must call billing-service/gRPC`);
		}
		if (!content.includes("run_billing_platform_action")) {
			errors.push(`${file}: Billing Platform mutations must use Billing gRPC actions`);
		}
	}

	for (const file of [
		"apps/backoffice-service/src/internal_admin.billing.admin.mutations.rs",
		"apps/backoffice-service/src/internal_admin.billing.admin.financial_actions.rs",
	]) {
		if (!existsSync(file)) continue;
		const content = readFileSync(file, "utf8");
		if (/\b(?:INSERT\s+INTO|UPDATE|DELETE\s+FROM)\s+billing_(?:credit_notes|write_offs|refunds|ledger_entries|adjustments)\b/i.test(content)) {
			errors.push(`${file}: Billing financial admin writes must call billing-service/gRPC`);
		}
	}

	const billingAdminMutations = "apps/backoffice-service/src/internal_admin.billing.admin.mutations.rs";
	if (existsSync(billingAdminMutations)) {
		const content = readFileSync(billingAdminMutations, "utf8");
		if (/\b(?:INSERT\s+INTO|UPDATE|DELETE\s+FROM)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${billingAdminMutations}: Billing admin mutations must call billing-service/gRPC`);
		}
	}

	const revenueMutations = "apps/backoffice-service/src/internal_admin.revenue_center.mutations.rs";
	if (existsSync(revenueMutations)) {
		const content = readFileSync(revenueMutations, "utf8");
		if (/\b(?:INSERT\s+INTO|UPDATE|DELETE\s+FROM)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${revenueMutations}: Revenue center Billing mutations must call billing-service/gRPC`);
		}
		if (!content.includes("run_revenue_grpc_action")) {
			errors.push(`${revenueMutations}: Revenue center mutations must use Billing gRPC actions`);
		}
	}
}

checkRustPackageBoundaries(errors);
checkProductSourceImports();
checkAccountBillingRuntimeBoundary(errors);
checkBillingWorkerQueueBoundaries(errors);
checkAccountWebBillingClientBoundary(errors);
checkDriveBillingBoundary();
checkDriveBillingMigrationBoundary();
checkInternalAdminBillingBoundary();
checkBillingMigrationBoundaries(errors);
checkBillingProviderEvidence(errors);
checkGatewayCloudBoundary(errors);
checkLegacyRuntimeNames(errors);

if (errors.length > 0) {
	console.error("Product boundary violations:");
	for (const error of errors) {
		console.error(`- ${error}`);
	}
	process.exit(1);
}

console.log("Product boundaries: ok");
