import { Badge } from "../components/ui/badge";

export function AccessReviewsPageHeader({
	activeCampaigns,
	pendingItems,
}: {
	activeCampaigns: number;
	pendingItems: number;
}) {
	return (
		<header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
			<div>
				<div className="flex items-center gap-2">
					<h1 className="text-2xl font-heading font-semibold">Access reviews</h1>
					<Badge variant="outline" className="rounded-md">
						Campaigns
					</Badge>
				</div>
				<p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
					Periodic certification for tenant members, assigned roles, service
					accounts, and OAuth clients.
				</p>
			</div>
			<div className="grid grid-cols-2 gap-2 text-right text-sm">
				<span className="text-muted-foreground">Active campaigns</span>
				<strong>{activeCampaigns}</strong>
				<span className="text-muted-foreground">Pending items</span>
				<strong>{pendingItems}</strong>
			</div>
		</header>
	);
}
