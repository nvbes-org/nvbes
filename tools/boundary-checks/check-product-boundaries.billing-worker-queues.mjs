import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

export function checkBillingWorkerQueueBoundaries(errors) {
	checkIdentityWorkerQueues(errors);
	checkIdentityQueueStatusSurface(errors);
	checkBillingWorkerQueues(errors);
}

const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);
const textExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".sql"]);

function walk(dir, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			if (!skippedDirs.has(entry)) walk(path, results);
			continue;
		}
		const relativePath = relative(process.cwd(), path).replaceAll("\\", "/");
		const dot = relativePath.lastIndexOf(".");
		if (dot >= 0 && textExtensions.has(relativePath.slice(dot))) results.push(relativePath);
	}
	return results;
}

function checkIdentityWorkerQueues(errors) {
	const loopPath = "apps/identity-worker/src/identity.worker.loop.rs";
	if (!existsSync(loopPath)) {
		errors.push(`${loopPath}: Identity worker queue declaration is required`);
		return;
	}
	const content = readFileSync(loopPath, "utf8");
	for (const expected of ["JOB_EMAIL_SEND", "JOB_EMAIL_WEBHOOK_PROCESS", "JOB_DATA_EXPORT"]) {
		if (!content.includes(expected)) {
			errors.push(`${loopPath}: Identity worker must keep owning identity queue ${expected}`);
		}
	}
	if (!content.includes("const WORKER_QUEUES: [&str; 3]")) {
		errors.push(`${loopPath}: Identity worker queues must stay explicit and bounded`);
	}
	if (!content.includes("identity_worker_queues_exclude_billing_runtime")) {
		errors.push(`${loopPath}: Identity worker queue boundary test is required`);
	}
	for (const forbidden of [
		"JOB_STRIPE_WEBHOOK_PROCESS",
		"JOB_MOLLIE_WEBHOOK_PROCESS",
		"JOB_BILLING_EMAIL_SEND",
		"billing.stripe.webhook.process",
		"billing.mollie.webhook.process",
		"billing.email.send",
	]) {
		if (content.includes(forbidden)) {
			errors.push(`${loopPath}: Identity worker must not claim Billing queue ${forbidden}`);
		}
	}
	for (const file of walk("apps/identity-worker/src")) {
		const content = readFileSync(file, "utf8");
		for (const forbidden of [
			"JOB_STRIPE_WEBHOOK_PROCESS",
			"JOB_MOLLIE_WEBHOOK_PROCESS",
			"JOB_BILLING_EMAIL_SEND",
			"process_stripe_event",
			"process_mollie_payment_update_tx",
			"enqueue_billing_email",
			"publish_workspace_billing_updates",
			"process_due_dunning_attempts",
			"billing.stripe.webhook.process",
			"billing.mollie.webhook.process",
			"billing.email.send",
		]) {
			if (content.includes(forbidden)) {
				errors.push(`${file}: Identity worker must not process Billing runtime ${forbidden}`);
			}
		}
	}
	const jobsPath = "apps/identity-worker/src/identity.worker.jobs.rs";
	if (existsSync(jobsPath) && !readFileSync(jobsPath, "utf8").includes("identity_worker_retries_only_identity_jobs")) {
		errors.push(`${jobsPath}: Identity worker retry boundary test is required`);
	}
}

function checkIdentityQueueStatusSurface(errors) {
	const servicePath = "apps/identity-api/src/identity.domains.security.service.rs";
	if (!existsSync(servicePath)) {
		errors.push(`${servicePath}: Identity queue status service is required`);
		return;
	}
	const serviceContent = readFileSync(servicePath, "utf8");
	for (const expected of ["use crate::email::jobs::JOB_EMAIL_SEND", "queue_status(redis, JOB_EMAIL_SEND)", "queue_name: JOB_EMAIL_SEND.to_string()"]) {
		if (!serviceContent.includes(expected)) {
			errors.push(`${servicePath}: Identity queue status must expose only the email queue evidence ${expected}`);
		}
	}
	for (const forbidden of ["billing.", "JOB_BILLING_EMAIL_SEND", "JOB_STRIPE_WEBHOOK_PROCESS", "JOB_MOLLIE_WEBHOOK_PROCESS"]) {
		if (serviceContent.includes(forbidden)) {
			errors.push(`${servicePath}: Identity queue status must not expose Billing queue ${forbidden}`);
		}
	}

	const routesPath = "apps/identity-api/src/identity.domains.security.routes.rs";
	if (existsSync(routesPath) && !readFileSync(routesPath, "utf8").includes("Identity email worker queue status")) {
		errors.push(`${routesPath}: Identity queue status OpenAPI description must identify the email queue boundary`);
	}
}

function checkBillingWorkerQueues(errors) {
	const jobsPath = "apps/billing-worker/src/billing.worker.jobs.rs";
	if (!existsSync(jobsPath)) {
		errors.push(`${jobsPath}: Billing worker queue declaration is required`);
		return;
	}
	const content = readFileSync(jobsPath, "utf8");
	for (const expected of [
		"const BILLING_QUEUES: [&str; 3]",
		"JOB_STRIPE_WEBHOOK_PROCESS",
		"JOB_MOLLIE_WEBHOOK_PROCESS",
		"JOB_BILLING_EMAIL_SEND",
		"claim_next_job(redis, &BILLING_QUEUES, 5)",
		"billing_worker_queues_are_exclusively_billing_runtime",
		"billing_worker_retries_only_billing_runtime_jobs",
	]) {
		if (!content.includes(expected)) {
			errors.push(`${jobsPath}: Billing worker must own Billing queue evidence ${expected}`);
		}
	}
}
