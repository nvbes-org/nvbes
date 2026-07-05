import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);
const textExtensions = new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".html", ".md"]);

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
			continue;
		}
		const relativePath = normalizePath(relative(process.cwd(), path));
		if (shouldScan(relativePath)) results.push(relativePath);
	}
	return results;
}

export function checkIdentityWebBillingClientBoundary(errors) {
	checkDedicatedBillingClient(errors);
	checkIdentityWebSourceDoesNotUseIdentityBilling(errors);
	checkIdentityWebDoesNotExposeBillingRuntime(errors);
	checkIdentityWebDoesNotEmitBillingAnalytics(errors);
	checkIdentityWebDoesNotLoadPspVendors(errors);
	checkBillingClientDoesNotExposeRawProviderIds(errors);
	checkIdentityClientDoesNotExposeBillingTransport(errors);
}

function checkDedicatedBillingClient(errors) {
	const billingClientPath = "apps/identity-web/src/billing.client.ts";
	if (!existsSync(billingClientPath)) {
		errors.push(`${billingClientPath}: Identity Web must centralize Billing transport through a dedicated Billing client`);
		return;
	}

	const content = readFileSync(billingClientPath, "utf8");
	if (!content.includes("@nvbes/billing-client")) {
		errors.push(`${billingClientPath}: Identity Web Billing transport must use @nvbes/billing-client`);
	}
	if (!content.includes("VITE_BILLING_API_BASE_URL")) {
		errors.push(`${billingClientPath}: Identity Web Billing transport must be configured by VITE_BILLING_API_BASE_URL`);
	}
	if (!content.includes("resolveBillingApiBaseUrl") || !content.includes("import.meta.env.DEV")) {
		errors.push(`${billingClientPath}: Identity Web Billing transport must fail closed outside local development`);
	}
	if (/VITE_BILLING_API_BASE_URL\s*\|\|/.test(content)) {
		errors.push(`${billingClientPath}: Billing API base URL must not use an implicit fallback`);
	}
	if (/VITE_IDENTITY_API_BASE_URL/.test(content) || /@nvbes\/identity-client/.test(content)) {
		errors.push(`${billingClientPath}: Billing transport must not fall back to Identity API/client`);
	}
}

function checkIdentityWebSourceDoesNotUseIdentityBilling(errors) {
	const rawProviderIds = [
		"provider_customer_id",
		"provider_payment_method_id",
		"provider_invoice_id",
		"provider_subscription_id",
		"provider_price_id",
		"provider_product_id",
		"stripe_customer_id",
		"stripe_price_id",
		"billing_customer_id",
		"billing_subscription_id",
	];

	for (const file of walk("apps/identity-web/src")) {
		const content = readFileSync(file, "utf8");
		if (/\bidentityClient\.(?:getBilling|createBilling)[A-Za-z0-9_]*/.test(content)) {
			errors.push(`${file}: Identity Web must use the Billing client for Billing calls`);
		}
		for (const forbidden of rawProviderIds) {
			if (content.includes(forbidden)) {
				errors.push(`${file}: Identity Web must not expose or track raw Billing provider identifier ${forbidden}`);
			}
		}
	}
}

function checkIdentityWebDoesNotExposeBillingRuntime(errors) {
	const forbiddenRuntimeCopy = [/worker billing/i, /billing worker/i, /file de paiement/i];
	for (const file of walk("apps/identity-web/src")) {
		const content = readFileSync(file, "utf8");
		for (const forbidden of forbiddenRuntimeCopy) {
			if (forbidden.test(content)) {
				errors.push(`${file}: Identity Web must not expose Billing runtime supervision copy ${forbidden}`);
			}
		}
	}
}

function checkIdentityWebDoesNotEmitBillingAnalytics(errors) {
	for (const file of walk("apps/identity-web/src")) {
		const content = readFileSync(file, "utf8");
		if (/trackEvent\(\s*["']billing\./.test(content)) {
			errors.push(`${file}: Identity Web must not emit Billing analytics events; Billing runtime owns Billing telemetry`);
		}
	}
}

function checkIdentityWebDoesNotLoadPspVendors(errors) {
	const files = [
		...walk("apps/identity-web/src"),
		"apps/identity-web/README.md",
		"apps/identity-web/index.html",
		"apps/identity-web/vite.config.ts",
		"apps/identity-web/identity.vite.csp.ts",
	];
	const forbiddenValues = [
		"VITE_STRIPE_",
		"https://js.stripe.com",
		"https://api.stripe.com",
		"js.stripe.com",
		"api.stripe.com",
		"Stripe",
	];

	for (const file of files) {
		if (!existsSync(file)) continue;
		const content = readFileSync(file, "utf8");
		for (const forbidden of forbiddenValues) {
			if (content.includes(forbidden)) {
				errors.push(`${file}: Identity Web must not load or configure PSP vendor endpoints directly; use Billing service routes`);
			}
		}
	}
}

function checkBillingClientDoesNotExposeRawProviderIds(errors) {
	const forbiddenProviderIds = [
		"provider_customer_id",
		"provider_payment_method_id",
		"provider_invoice_id",
		"provider_subscription_id",
		"provider_price_id",
		"provider_product_id",
		"stripe_customer_id",
		"stripe_price_id",
		"billing_customer_id",
		"billing_subscription_id",
		"session_id",
		"checkout_id",
		"payment_id",
	];

	for (const file of walk("libs/ts/billing-client/src")) {
		const content = readFileSync(file, "utf8");
		for (const forbidden of forbiddenProviderIds) {
			if (content.includes(forbidden)) {
				errors.push(`${file}: Billing TypeScript client must not expose raw provider identifier ${forbidden}`);
			}
		}
	}
}

function checkIdentityClientDoesNotExposeBillingTransport(errors) {
	const accountClient = "libs/ts/identity-client/src/identity.account-client.ts";
	if (existsSync(accountClient)) {
		const content = readFileSync(accountClient, "utf8");
		if (/\/workspaces\/.*\/billing\//.test(content) || /\b(?:getBilling|createBilling)[A-Za-z0-9_]*/.test(content)) {
			errors.push(`${accountClient}: AccountIdentityClient must not expose Billing transport methods`);
		}
	}

	for (const file of walk("libs/ts/identity-client/src")) {
		const content = readFileSync(file, "utf8");
		if (/billing\.(?:client|provider|schemas)/.test(file) || /@nvbes\/billing-client/.test(content)) {
			errors.push(`${file}: Identity client package must not own or re-export Billing client surfaces`);
		}
		if (/\/enterprise\/billing\b/.test(content) || /\bEnterpriseBilling[A-Za-z0-9_]*\b/.test(content)) {
			errors.push(`${file}: Identity client package must not expose Enterprise Billing transport surfaces`);
		}
	}
}
