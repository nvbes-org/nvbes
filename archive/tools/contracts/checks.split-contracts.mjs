import { existsSync, readFileSync } from "node:fs";

const splitContracts = [
	{
		path: "contracts/protobuf/nvbes/account/v1/account.proto",
		service: "AccountService",
		required: [
			"RegisterUser",
			"BeginPasswordSignIn",
			"CompletePasswordSignIn",
			"StartMfaChallenge",
			"VerifyMfaChallenge",
			"ListSessions",
			"ExchangeToken",
			"GetAccountBillingOverview",
			"CreateAccountCheckout",
		],
	},
	{
		path: "contracts/protobuf/nvbes/cloud/v1/cloud.proto",
		service: "CloudService",
		required: [
			'reserved 7;',
			"CreateWorkspace",
			"AddWorkspaceMember",
			"CreateUpload",
			"CompleteUpload",
			"GetDownloadUrl",
			"GetWorkspaceQuota",
			"ReserveStorage",
			"DriveObject",
			"google.protobuf.Timestamp",
			"enum DriveObjectKind",
			"enum DriveObjectStatus",
		],
		forbidden: ["storage_key"],
	},
	{
		path: "contracts/protobuf/nvbes/developer/v1/developer.proto",
		service: "DeveloperService",
		required: [
			"CreateApp",
			"RotateAppSecret",
			"CreateWebhookEndpoint",
			"ReplayWebhookDelivery",
			"CreateDeveloperToken",
			"CreateSandbox",
			"ListApiLogs",
		],
	},
	{
		path: "contracts/protobuf/nvbes/enterprise/v1/enterprise.proto",
		service: "EnterpriseService",
		required: [
			"UpdatePolicy",
			"EvaluatePolicy",
			"StartAccessReview",
			"ActivateBreakGlass",
			"ConfigureFederationProvider",
			"GetFederationGovernance",
		],
	},
	{
		path: "contracts/protobuf/nvbes/billing/v1/billing.proto",
		service: "BillingService",
		required: [
			"CreateCheckout",
			"GetEntitlements",
			"ListInvoices",
			"RecordLedgerEntry",
			"IngestProviderWebhook",
			"ReconcileProviderWebhook",
			"checkout_session_id",
		],
	},
];

export const requiredSplitEventTypes = [
	"account.user.created",
	"account.session.created",
	"account.token.revoked",
	"account.billing.facade.requested",
	"cloud.workspace.created",
	"cloud.workspace.membership.created",
	"cloud.workspace.quota.changed",
	"developer.app.created",
	"developer.webhook.endpoint.created",
	"developer.token.created",
	"enterprise.policy.changed",
	"enterprise.access_review.started",
	"enterprise.break_glass.activated",
	"enterprise.federation.provider.changed",
];

export function checkSplitProtoContracts(errors) {
	for (const contract of splitContracts) {
		if (!existsSync(contract.path)) {
			errors.push(`${contract.path}: missing split service contract`);
			continue;
		}
		const content = readFileSync(contract.path, "utf8");
		if (!content.includes(`service ${contract.service}`)) {
			errors.push(`${contract.path}: missing ${contract.service}`);
		}
		if (!content.includes('import "nvbes/platform/v1/common.proto";')) {
			errors.push(`${contract.path}: must import platform common proto`);
		}
		for (const expected of contract.required) {
			if (!content.includes(expected)) {
				errors.push(`${contract.path}: missing ${expected}`);
			}
		}
		for (const forbidden of contract.forbidden ?? []) {
			if (content.includes(forbidden)) {
				errors.push(`${contract.path}: forbidden cross-service field ${forbidden}`);
			}
		}
		for (const match of content.matchAll(/message\s+([A-Za-z0-9]+Request)\s*\{([\s\S]*?)\n\}/g)) {
			const [, messageName, body] = match;
			if (!body.includes("nvbes.platform.v1.RequestContext context = 1;")) {
				errors.push(`${contract.path}: ${messageName} must carry RequestContext as field 1`);
			}
		}
	}
}
