import type {
	EnterpriseModuleGrant,
	EnterpriseRole,
} from "@nvbes/identity-client";

const tenantOnlyEnterpriseNavItems = new Set([
	"Developers",
	"Policies",
	"Security",
	"Access reviews",
	"Billing",
	"Settings",
]);

const grantLabels: Record<EnterpriseModuleGrant, string> = {
	audit: "Audit logs",
	billing: "Billing",
	developers: "Developers",
	drive: "Drive",
	members: "Members",
	policies: "Policies",
	security: "Security",
	workspaces: "Workspaces",
};

export function canManageUsers(input: {
	role: EnterpriseRole;
	module_grants: EnterpriseModuleGrant[];
}): boolean {
	return (
		input.role === "owner" ||
		(input.role === "admin" && input.module_grants.includes("members"))
	);
}

export function canManagePolicies(input: {
	role: EnterpriseRole;
	module_grants: EnterpriseModuleGrant[];
}): boolean {
	return (
		input.role === "owner" ||
		(input.role === "admin" && input.module_grants.includes("policies"))
	);
}

export function isTenantOnlyEnterpriseNavItem(label: string): boolean {
	return tenantOnlyEnterpriseNavItems.has(label);
}

export function isLastOwnerRemoval(input: {
	currentOwnerCount: number;
	selectedRole: EnterpriseRole;
	nextRole: EnterpriseRole;
}): boolean {
	return (
		input.currentOwnerCount <= 1 &&
		input.selectedRole === "owner" &&
		input.nextRole !== "owner"
	);
}

export function describeModuleGrant(grant: EnterpriseModuleGrant): string {
	return grantLabels[grant];
}
