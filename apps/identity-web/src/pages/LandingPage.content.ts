import {
	BarChart3,
	ClipboardList,
	DollarSign,
	FileText,
	Folder,
	ShieldCheck,
	UserRound,
	UserRoundPlus,
} from "lucide-react";
import type { ComponentType, SVGProps } from "react";

type Icon = ComponentType<SVGProps<SVGSVGElement>>;

export const capabilities: Array<{
	title: string;
	body: string;
	icon: Icon;
	points: string[];
}> = [
	{
		title: "Secure identity & access",
		body: "Verify every user and device. Enforce least-privilege access with policy that scales.",
		icon: UserRound,
		points: [
			"SSO, SCIM, and granular roles",
			"MFA, device posture, and session controls",
			"Attribute- and context-based access",
		],
	},
	{
		title: "Private workspace storage",
		body: "Store and share files within encrypted workspaces built for product teams.",
		icon: FileText,
		points: [
			"Server-side encryption with per-workspace keys",
			"Fine-grained permissions and link controls",
			"Versioning, restore, and secure delete",
		],
	},
	{
		title: "Audit trails & billing",
		body: "Immutable audit logs and transparent billing keep your team accountable and in control.",
		icon: BarChart3,
		points: [
			"Real-time audit stream and export",
			"Usage-based billing with clear invoices",
			"Budgets, alerts, and seat management",
		],
	},
];

export const workflow = [
	{
		title: "Invite",
		body: "Add teammates or sync from your identity provider.",
		icon: UserRoundPlus,
	},
	{
		title: "Grant access",
		body: "Apply role-based policies to the right workspaces.",
		icon: ShieldCheck,
	},
	{
		title: "Work securely",
		body: "Create, share, and store files with encrypted storage.",
		icon: Folder,
	},
	{
		title: "Audit",
		body: "Track events in real time with immutable audit logs.",
		icon: ClipboardList,
	},
	{
		title: "Bill & scale",
		body: "Monitor usage, set budgets, and scale with confidence.",
		icon: DollarSign,
	},
];
