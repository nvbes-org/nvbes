import type {
	AccessReviewCampaignSummary,
	AccessReviewItem,
} from "@nvbes/identity-client";
import { Badge } from "../components/ui/badge";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "../components/ui/table";

export function CampaignTable({
	campaigns,
	selectedCampaignId,
	onSelect,
}: {
	campaigns: AccessReviewCampaignSummary[];
	selectedCampaignId: string | null;
	onSelect: (campaignId: string) => void;
}) {
	if (campaigns.length === 0) {
		return (
			<div className="flex min-h-44 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
				No access review campaigns yet.
			</div>
		);
	}
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Name</TableHead>
					<TableHead>Status</TableHead>
					<TableHead>Due</TableHead>
					<TableHead className="text-right">Pending</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{campaigns.map((campaign) => (
					<TableRow
						key={campaign.id}
						data-state={
							campaign.id === selectedCampaignId ? "selected" : undefined
						}
						onClick={() => onSelect(campaign.id)}
						className="cursor-pointer"
					>
						<TableCell className="font-medium">{campaign.name}</TableCell>
						<TableCell>
							<Badge
								variant={campaign.status === "active" ? "default" : "outline"}
							>
								{campaign.status}
							</Badge>
						</TableCell>
						<TableCell>{formatDate(campaign.due_at)}</TableCell>
						<TableCell className="text-right">
							{campaign.pending_items}
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	);
}

export function ReviewItemsTable({ items }: { items: AccessReviewItem[] }) {
	if (items.length === 0) {
		return (
			<div className="flex min-h-44 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
				Select or create a campaign to inspect review items.
			</div>
		);
	}
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Subject</TableHead>
					<TableHead>Type</TableHead>
					<TableHead>Role</TableHead>
					<TableHead>Status</TableHead>
					<TableHead>Decision</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{items.map((item) => (
					<TableRow key={item.id}>
						<TableCell className="font-medium">{item.subject_label}</TableCell>
						<TableCell>{item.item_type}</TableCell>
						<TableCell>{item.role ?? "none"}</TableCell>
						<TableCell>{item.status}</TableCell>
						<TableCell>
							<Badge
								variant={item.decision === "pending" ? "outline" : "secondary"}
							>
								{item.decision}
							</Badge>
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	);
}

function formatDate(value: string) {
	return new Intl.DateTimeFormat(undefined, {
		day: "2-digit",
		month: "short",
		year: "numeric",
	}).format(new Date(value));
}
