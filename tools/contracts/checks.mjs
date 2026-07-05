#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const errors = [];

function readJson(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return undefined;
	}

	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function walk(dir, predicate, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			walk(path, predicate, results);
		} else if (predicate(path)) {
			results.push(path);
		}
	}
	return results;
}

function routePathsFromSource(dir, api) {
	const files = walk(dir, (path) => path.endsWith(".rs"));
	const paths = new Map();

	for (const file of files) {
		const content = readFileSync(file, "utf8");
		for (const match of content.matchAll(/path\s*=\s*"([^"]+)"/g)) {
			const path = match[1];
			if (!path.startsWith("/")) continue;
			if (api.versionPrefix && !path.startsWith(api.versionPrefix)) continue;
			if (!paths.has(path)) paths.set(path, []);
			paths.get(path).push(file);
		}
	}

	return paths;
}

function checkOpenApi() {
	const manifest = readJson("contracts/openapi/manifest.json");
	if (!manifest) return;
	if (!Array.isArray(manifest.apis) || manifest.apis.length === 0) {
		errors.push("contracts/openapi/manifest.json: apis must be a non-empty array");
		return;
	}

	for (const api of manifest.apis) {
		if (!api.name || !api.document || !api.surface) {
			errors.push("contracts/openapi/manifest.json: every api needs name, surface and document");
			continue;
		}

		const spec = readJson(api.document);
		if (!spec) continue;
		if (typeof spec.openapi !== "string" || !spec.openapi.startsWith("3.")) {
			errors.push(`${api.document}: OpenAPI 3.x document expected`);
		}
		if (!spec.info?.title || !spec.info?.version) {
			errors.push(`${api.document}: info.title and info.version are required`);
		}
		const paths = Object.keys(spec.paths ?? {});
		if (paths.length === 0) {
			errors.push(`${api.document}: at least one path is required`);
		}
		if (api.surface === "public" && api.versionPrefix) {
			if (!paths.some((path) => path.startsWith(api.versionPrefix))) {
				errors.push(`${api.document}: public API must expose ${api.versionPrefix} paths`);
			}
		}
		if (api.routeSource) {
			const sourcePaths = routePathsFromSource(api.routeSource, api);
			const specPaths = new Set(paths);
			for (const [path, files] of sourcePaths.entries()) {
				if (!specPaths.has(path)) {
					errors.push(
						`${api.document}: route ${path} from ${files[0]} is missing from OpenAPI`,
					);
				}
			}
		}
	}
}

function checkProto() {
	const files = walk("contracts/protobuf", (path) => path.endsWith(".proto"));
	if (files.length === 0) {
		errors.push("contracts/protobuf: at least one .proto file is required");
	}

	for (const file of files) {
		const content = readFileSync(file, "utf8");
		if (!content.includes('syntax = "proto3";')) {
			errors.push(`${file}: proto3 syntax is required`);
		}
		if (!/^package\s+nvbes\.[a-z0-9_.]+\.v\d+;/m.test(content)) {
			errors.push(`${file}: package must be versioned under nvbes.*.vN`);
		}
		if (/\bTODO\b|\bTBD\b|\bFIXME\b/i.test(content)) {
			errors.push(`${file}: unresolved marker`);
		}
	}
}

function checkBillingGrpcImplementation() {
	const protoPath = "contracts/protobuf/nvbes/billing/v1/billing.proto";
	if (!existsSync(protoPath)) return;
	const proto = readFileSync(protoPath, "utf8");
	if (!proto.includes("service BillingService")) return;

	const requiredFiles = [
		"apps/billing-api/build.rs",
		"apps/billing-api/Cargo.toml",
		"apps/billing-api/src/billing.grpc.pb.rs",
		"apps/billing-api/src/billing.grpc.service.rs",
		"apps/billing-api/src/main.rs",
	];
	for (const file of requiredFiles) {
		if (!existsSync(file)) errors.push(`${file}: required for BillingService gRPC transport`);
	}
	if (errors.length > 0) return;

	const build = readFileSync("apps/billing-api/build.rs", "utf8");
	if (!build.includes("tonic_prost_build::configure().compile_protos")) {
		errors.push("apps/billing-api/build.rs: BillingService must be generated with tonic/prost");
	}
	if (!build.includes("contracts/protobuf/nvbes/billing/v1/billing.proto")) {
		errors.push("apps/billing-api/build.rs: BillingService proto source must be generated");
	}

	const manifest = readFileSync("apps/billing-api/Cargo.toml", "utf8");
	for (const dependency of ["tonic", "tonic-prost", "tonic-prost-build"]) {
		if (!manifest.includes(dependency)) {
			errors.push(`apps/billing-api/Cargo.toml: missing ${dependency} for BillingService gRPC`);
		}
	}

	const pb = readFileSync("apps/billing-api/src/billing.grpc.pb.rs", "utf8");
	if (!pb.includes('tonic::include_proto!("nvbes.billing.v1")')) {
		errors.push("apps/billing-api/src/billing.grpc.pb.rs: missing nvbes.billing.v1 include");
	}

	const service = readFileSync("apps/billing-api/src/billing.grpc.service.rs", "utf8");
	for (const expected of [
		"impl BillingService for BillingGrpcService",
		"fetch_workspace_billing_overview",
		"fetch_portal_view",
		"create_billing_checkout_session",
		"create_billing_portal_session",
		"get_admin_command_center_billing_metrics",
		"get_admin_operations_center",
		"run_admin_operations_action",
		"run_admin_billing_platform_action",
		"run_admin_billing_action",
		"run_admin_revenue_action",
		"ingest_usage_event",
		"run_ledger_reconciliation",
	]) {
		if (!service.includes(expected)) {
			errors.push(`apps/billing-api/src/billing.grpc.service.rs: missing ${expected}`);
		}
	}

	for (const expected of [
		"ADMIN_BILLING_ACTION_KIND_CREATE_CREDIT_NOTE",
		"ADMIN_BILLING_ACTION_KIND_CREATE_WRITE_OFF",
		"ADMIN_BILLING_ACTION_KIND_CREATE_REFUND_INTENT",
		"ADMIN_BILLING_ACTION_KIND_CREATE_MANUAL_COMPENSATION",
		"ADMIN_BILLING_ACTION_KIND_REPLAY_PROVIDER_EVENT",
		"ADMIN_BILLING_ACTION_KIND_CREATE_PROVIDER_MIGRATION",
		"ADMIN_BILLING_ACTION_KIND_OVERRIDE_GRACE_PERIOD",
		"ADMIN_BILLING_PLATFORM_ACTION_KIND_APPROVE_FRAUD_ASSESSMENT",
		"ADMIN_BILLING_PLATFORM_ACTION_KIND_REJECT_FRAUD_ASSESSMENT",
		"ADMIN_BILLING_PLATFORM_ACTION_KIND_TRUST_FRAUD_ASSESSMENT",
		"ADMIN_REVENUE_ACTION_KIND_CLOSE_DUNNING_CASE",
		"ADMIN_REVENUE_ACTION_KIND_REOPEN_DUNNING_CASE",
		"ADMIN_REVENUE_ACTION_KIND_HOLD_INVOICE",
		"ADMIN_REVENUE_ACTION_KIND_RELEASE_INVOICE",
		"ADMIN_REVENUE_ACTION_KIND_REVIEW_DISPUTE",
		"ADMIN_REVENUE_ACTION_KIND_RESOLVE_DISPUTE",
	]) {
		if (!proto.includes(expected)) {
			errors.push(`${protoPath}: missing ${expected}`);
		}
	}

	const main = readFileSync("apps/billing-api/src/main.rs", "utf8");
	if (!main.includes("NVBES_BILLING_GRPC_PORT") || !main.includes("nvbes_billing_api::grpc::serve")) {
		errors.push("apps/billing-api/src/main.rs: Billing gRPC server must be started with a dedicated port");
	}
}

function checkBillingPublicWorkspaceRoutes() {
	const billingRoutes = "apps/billing-api/src/billing.domains.public_workspace.rs";
	const billingPortalLists = "apps/billing-api/src/billing.domains.public_workspace.portal_lists.rs";
	if (!existsSync(billingRoutes)) {
		errors.push(`${billingRoutes}: required for public Billing workspace routes`);
		return;
	}
	if (!existsSync(billingPortalLists)) {
		errors.push(`${billingPortalLists}: required for public Billing portal list routes`);
		return;
	}

	const content = readFileSync(billingRoutes, "utf8");
	const surface = `${content}\n${readFileSync(billingPortalLists, "utf8")}`;
	for (const route of [
		'"/billing/portal/capabilities"',
		'"/workspaces/{workspaceId}/billing/overview"',
		'"/workspaces/{workspaceId}/billing/usage"',
		'"/workspaces/{workspaceId}/billing/entitlements"',
		'"/workspaces/{workspaceId}/billing/checkout"',
		'"/workspaces/{workspaceId}/billing/portal"',
		'"/workspaces/{workspaceId}/billing/portal/view"',
		'"/workspaces/{workspaceId}/billing/invoices"',
		'"/workspaces/{workspaceId}/billing/cards"',
		'"/workspaces/{workspaceId}/billing/subscriptions"',
		'"/workspaces/{workspaceId}/billing/portal/invoices/{invoiceId}/pdf"',
	]) {
		if (!content.includes(route)) {
			errors.push(`${billingRoutes}: missing public Billing route ${route}`);
		}
	}

	for (const expected of [
		"fetch_workspace_billing_overview",
		"fetch_workspace_billing_usage",
		"fetch_workspace_entitlements",
		"create_billing_checkout_session",
		"create_billing_portal_session",
		"fetch_portal_view",
		"fetch_portal_invoices",
		"fetch_portal_payment_methods",
		"fetch_portal_subscription_providers",
		"fetch_invoice_pdf",
		"BillingWorkspacePermission::Read",
		"BillingWorkspacePermission::Manage",
	]) {
		if (!surface.includes(expected)) {
			errors.push(`apps/billing-api/src: missing public Billing route evidence ${expected}`);
		}
	}
}

function checkGraphql() {
	const schemaPath = "contracts/graphql/schema.graphql";
	const governancePath = "contracts/graphql/governance.json";
	if (!existsSync(schemaPath)) {
		errors.push(`${schemaPath}: missing`);
		return;
	}
	const schema = readFileSync(schemaPath, "utf8");
	if (!schema.includes("schema {") || !schema.includes("type Query")) {
		errors.push(`${schemaPath}: executable schema and Query type are required`);
	}
	if (/\bTODO\b|\bTBD\b|\bFIXME\b/i.test(schema)) {
		errors.push(`${schemaPath}: unresolved marker`);
	}
	for (const forbidden of [
		"checkoutSessionId",
		"portalSessionId",
		"providerCustomerId",
		"providerPaymentMethodId",
		"providerInvoiceId",
		"providerSubscriptionId",
	]) {
		if (schema.includes(forbidden)) {
			errors.push(`${schemaPath}: ${forbidden} must not be exposed`);
		}
	}

	const governance = readJson(governancePath);
	if (!governance) return;
	if (governance.schema !== schemaPath) {
		errors.push(`${governancePath}: schema must point at ${schemaPath}`);
	}
	if (!Array.isArray(governance.owners) || governance.owners.length === 0) {
		errors.push(`${governancePath}: owners must be a non-empty array`);
	}
	if (!Array.isArray(governance.rules) || governance.rules.length === 0) {
		errors.push(`${governancePath}: rules must be a non-empty array`);
	}

	checkGraphqlGatewayImplementation(governance);
}

function checkGraphqlGatewayImplementation(governance) {
	if (!governance.entrypoints?.includes("gateway-graphql")) {
		errors.push("contracts/graphql/governance.json: gateway-graphql entrypoint is required");
	}

	const requiredFiles = [
		"apps/gateway-graphql/Cargo.toml",
		"apps/gateway-graphql/build.rs",
		"apps/gateway-graphql/src/gateway.pb.rs",
		"apps/gateway-graphql/src/gateway.billing_client.rs",
		"apps/gateway-graphql/src/gateway.auth.rs",
		"apps/gateway-graphql/src/gateway.schema.rs",
		"apps/gateway-graphql/src/gateway.schema.types.rs",
		"apps/gateway-graphql/src/main.rs",
	];
	for (const file of requiredFiles) {
		if (!existsSync(file)) errors.push(`${file}: required for GraphQL gateway`);
	}
	if (requiredFiles.some((file) => !existsSync(file))) return;

	const manifest = readFileSync("apps/gateway-graphql/Cargo.toml", "utf8");
	for (const dependency of ["async-graphql", "async-graphql-axum", "tonic", "tonic-prost-build"]) {
		if (!manifest.includes(dependency)) {
			errors.push(`apps/gateway-graphql/Cargo.toml: missing ${dependency}`);
		}
	}

	const build = readFileSync("apps/gateway-graphql/build.rs", "utf8");
	if (!build.includes("contracts/protobuf/nvbes/billing/v1/billing.proto")) {
		errors.push("apps/gateway-graphql/build.rs: must generate BillingService proto");
	}

	const billingClient = readFileSync("apps/gateway-graphql/src/gateway.billing_client.rs", "utf8");
	for (const expected of ["Endpoint::from_shared", ".connect_timeout(", ".timeout("]) {
		if (!billingClient.includes(expected)) {
			errors.push(`apps/gateway-graphql/src/gateway.billing_client.rs: gRPC client must configure ${expected}`);
		}
	}

	const auth = readFileSync("apps/gateway-graphql/src/gateway.auth.rs", "utf8");
	for (const expected of ["x-nvbes-actor-principal-id", "x-nvbes-tenant-id", "Uuid::parse_str"]) {
		if (!auth.includes(expected)) {
			errors.push(`apps/gateway-graphql/src/gateway.auth.rs: Identity-authenticated gateway header ${expected} is required`);
		}
	}

	const schemaSource = readFileSync("apps/gateway-graphql/src/gateway.schema.rs", "utf8");
	for (const expected of ["for_workspace(&workspace_id)", "for_workspace(&input.workspace_id)", "context: Some(request_context)"]) {
		if (!schemaSource.includes(expected)) {
			errors.push(`apps/gateway-graphql/src/gateway.schema.rs: Billing resolvers must propagate ${expected} to gRPC`);
		}
	}
	const schemaTypes = readFileSync("apps/gateway-graphql/src/gateway.schema.types.rs", "utf8");
	if (!schemaTypes.includes("billing_provider_reference_exposes_only_neutral_provider_state")) {
		errors.push("apps/gateway-graphql/src/gateway.schema.types.rs: Billing provider reference mapping test is required");
	}
	if (!schemaTypes.includes("billing_session_types_expose_urls_without_raw_provider_ids")) {
		errors.push("apps/gateway-graphql/src/gateway.schema.types.rs: Billing session URL mapping test is required");
	}

	const gatewayRustFiles = walk("apps/gateway-graphql/src", (path) => path.endsWith(".rs"));
	const source = gatewayRustFiles
		.map((file) => readFileSync(file, "utf8"))
		.join("\n");
	for (const expected of [
		"get_billing_overview",
		"get_billing_portal",
		"create_checkout",
		"create_portal",
		"GatewayRequestContext",
	]) {
		if (!source.includes(expected)) {
			errors.push(`apps/gateway-graphql: missing GraphQL gateway resolver evidence ${expected}`);
		}
	}
	for (const forbidden of ["reqwest::", "hyper::Client", "billing_api_base_url", "/workspaces/{workspaceId}/billing"]) {
		if (source.includes(forbidden)) {
			errors.push(`apps/gateway-graphql: Billing gateway must use gRPC only, found ${forbidden}`);
		}
	}
	const graphqlSchema = readFileSync("contracts/graphql/schema.graphql", "utf8");
	for (const forbidden of [
		"checkout_session_id",
		"portal_session_id",
		"provider_session_id",
		"provider_customer_id",
		"provider_payment_method_id",
		"provider_invoice_id",
		"provider_subscription_id",
		"provider_event_id",
		"provider_product_id",
		"provider_price_id",
		"provider_mandate_id",
		"provider_attempt_id",
		"stripe_customer_id",
		"stripe_subscription_id",
		"stripe_payment_method_id",
		"stripe_invoice_id",
		"stripe_price_id",
		"stripe_product_id",
		"mollie_customer_id",
		"mollie_mandate_id",
		"mollie_payment_id",
		"mollie_subscription_id",
		"cb_card_id",
		"cb_token_id",
	]) {
		if (source.includes(forbidden)) {
			errors.push(`apps/gateway-graphql: must not expose raw PSP identifier ${forbidden}`);
		}
		if (graphqlSchema.includes(forbidden)) {
			errors.push(`contracts/graphql/schema.graphql: must not expose raw PSP identifier ${forbidden}`);
		}
	}
}

function checkEvents() {
	const manifest = readJson("contracts/events/manifest.json");
	const envelope = readJson("contracts/events/envelope.schema.json");
	if (!manifest || !envelope) return;

	const requiredEnvelopeFields = [
		"event_id",
		"event_type",
		"event_version",
		"tenant_id",
		"region_id",
		"occurred_at",
		"correlation_id",
		"idempotency_key",
		"payload",
	];
	for (const field of requiredEnvelopeFields) {
		if (!envelope.required?.includes(field)) {
			errors.push(`contracts/events/envelope.schema.json: missing required field ${field}`);
		}
	}

	if (!Array.isArray(manifest.events) || manifest.events.length === 0) {
		errors.push("contracts/events/manifest.json: events must be a non-empty array");
		return;
	}

	const seen = new Set();
	for (const event of manifest.events) {
		const key = `${event.event_type}@${event.event_version}`;
		if (seen.has(key)) {
			errors.push(`contracts/events/manifest.json: duplicate event ${key}`);
		}
		seen.add(key);

		if (!event.critical) {
			errors.push(`contracts/events/manifest.json: ${key} must declare critical=true or move out of this manifest`);
		}

		const schema = readJson(event.schema);
		if (!schema) continue;
		if (schema.properties?.event_type?.const !== event.event_type) {
			errors.push(`${event.schema}: event_type const must match manifest`);
		}
		if (schema.properties?.event_version?.const !== event.event_version) {
			errors.push(`${event.schema}: event_version const must match manifest`);
		}
		if (!schema.properties?.payload || !schema.required?.includes("payload")) {
			errors.push(`${event.schema}: payload property is required`);
		}
		const providerEnum = schema.properties?.payload?.properties?.provider?.enum;
		if (event.event_type.startsWith("billing.") && providerEnum) {
			for (const provider of ["stripe", "mollie", "cb"]) {
				if (!providerEnum.includes(provider)) {
					errors.push(`${event.schema}: billing provider enum must include ${provider}`);
				}
			}
		}
	}
}

checkOpenApi();
checkProto();
checkBillingGrpcImplementation();
checkBillingPublicWorkspaceRoutes();
checkGraphql();
checkEvents();

if (errors.length > 0) {
	console.error("Contract checks failed:");
	for (const error of errors) {
		console.error(`- ${error}`);
	}
	process.exit(1);
}

console.log("Contracts: ok");
