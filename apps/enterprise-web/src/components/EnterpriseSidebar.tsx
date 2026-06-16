import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
	Building2,
	ChartNoAxesCombined,
	ClipboardCheck,
	Code2,
	CreditCard,
	FileSearch,
	LayoutDashboard,
	ScrollText,
	Settings,
	Shield,
	Users,
} from "lucide-react";
import { useMemo, type ComponentType } from "react";
import { enterpriseContextQueryOptions } from "../enterprise.queries";
import { isTenantOnlyEnterpriseNavItem } from "../enterprise.permissions";

export const enterpriseNavSections = [
	{
		title: "Operate",
		items: ["Overview", "Users", "Workspaces", "Developers"],
	},
	{
		title: "Govern",
		items: ["Policies", "Security", "Access reviews", "Audit logs"],
	},
	{ title: "Commercial", items: ["Billing", "Usage", "Settings"] },
] as const;

type EnterpriseNavLabel =
	(typeof enterpriseNavSections)[number]["items"][number];

const enterpriseNavItems: Record<
	EnterpriseNavLabel,
	{ icon: ComponentType<{ className?: string }>; to: string; end?: boolean }
> = {
	Overview: { icon: LayoutDashboard, to: "/", end: true },
	Users: { icon: Users, to: "/users" },
	Workspaces: { icon: Building2, to: "/workspaces" },
	Developers: { icon: Code2, to: "/developers" },
	Policies: { icon: ScrollText, to: "/policies" },
	Security: { icon: Shield, to: "/security" },
	"Access reviews": { icon: ClipboardCheck, to: "/access-reviews" },
	"Audit logs": { icon: FileSearch, to: "/audit-logs" },
	Billing: { icon: CreditCard, to: "/billing" },
	Usage: { icon: ChartNoAxesCombined, to: "/usage" },
	Settings: { icon: Settings, to: "/settings" },
};

type EnterpriseSidebarProps = {
	variant?: "sidebar" | "mobile";
};

export function EnterpriseSidebar({
	variant = "sidebar",
}: EnterpriseSidebarProps) {
	const { data: context } = useQuery(enterpriseContextQueryOptions());

	const filteredNavSections = useMemo(() => {
		const isOrgScoped = !!context?.organization_id;
		return enterpriseNavSections
			.map((section) => {
				const items = section.items.filter((item) => {
					if (isOrgScoped) {
						return !isTenantOnlyEnterpriseNavItem(item);
					}
					return true;
				});
				return { ...section, items };
			})
			.filter((section) => section.items.length > 0);
	}, [context?.organization_id]);

	if (variant === "mobile") {
		return (
			<nav className="flex gap-2 overflow-x-auto px-3 py-2">
				{filteredNavSections.flatMap((section) =>
					section.items.map((label) => (
						<EnterpriseNavItem key={label} label={label} compact />
					)),
				)}
			</nav>
		);
	}

	return (
		<div className="flex h-full flex-col bg-background text-foreground">
			<div className="border-b border-border px-4 py-4">
				<p className="truncate text-sm font-heading font-semibold">
					nvbes Enterprise
				</p>
				<p className="mt-0.5 truncate text-xs text-muted-foreground">
					{context?.organization_id
						? "Organization administration"
						: "Tenant administration"}
				</p>
			</div>

			<nav className="flex min-w-0 flex-1 flex-col gap-5 overflow-y-auto px-3 py-4">
				{filteredNavSections.map((section) => (
					<div key={section.title} className="flex min-w-0 flex-col gap-1">
						<span className="truncate px-2.5 text-[0.7rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
							{section.title}
						</span>
						<div className="mt-1 flex flex-col gap-0.5">
							{section.items.map((label) => (
								<EnterpriseNavItem key={label} label={label} />
							))}
						</div>
					</div>
				))}
			</nav>
		</div>
	);
}

function EnterpriseNavItem({
	label,
	compact = false,
}: {
	label: EnterpriseNavLabel;
	compact?: boolean;
}) {
	const item = enterpriseNavItems[label];
	const Icon = item.icon;

	return (
		<Link
			to={item.to}
			activeOptions={{ exact: item.end }}
			className={
				compact
					? "flex h-9 shrink-0 items-center gap-2 rounded-lg px-3 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
					: "flex min-w-0 items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
			}
			activeProps={{ className: "bg-muted text-foreground" }}
		>
			<Icon className="size-4 shrink-0" />
			<span className={compact ? "whitespace-nowrap" : "truncate"}>
				{label}
			</span>
		</Link>
	);
}
