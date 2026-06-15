import type { CreateAccessReviewCampaignInput } from "@nvbes/identity-client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ClipboardCheck, RefreshCw } from "lucide-react";
import { useMemo, useState } from "react";
import { Alert, AlertDescription, AlertTitle } from "../components/ui/alert";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "../components/ui/card";
import { Checkbox } from "../components/ui/checkbox";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { Skeleton } from "../components/ui/skeleton";
import { enterpriseClient } from "../enterprise.api";
import {
	accessReviewCampaignsQueryOptions,
	enterpriseQueryKeys,
} from "../enterprise.queries";
import { CampaignTable, ReviewItemsTable } from "./AccessReviewsPage.tables";

const scopeOptions = [
	["include_members", "Members"],
	["include_roles", "Roles"],
	["include_service_accounts", "Service accounts"],
	["include_oauth_clients", "OAuth clients"],
] as const;

type ScopeKey = (typeof scopeOptions)[number][0];

const defaultScope: CreateAccessReviewCampaignInput["scope"] = {
	include_members: true,
	include_roles: true,
	include_service_accounts: true,
	include_oauth_clients: true,
};

export function AccessReviewsPage() {
	const queryClient = useQueryClient();
	const campaignsQuery = useQuery(accessReviewCampaignsQueryOptions());
	const [selectedCampaignId, setSelectedCampaignId] = useState<string | null>(
		null,
	);
	const [form, setForm] = useState(() => ({
		name: "Quarterly access review",
		dueDate: defaultDueDate(),
		scope: defaultScope,
	}));

	const campaigns = campaignsQuery.data?.campaigns ?? [];
	const selectedCampaign = useMemo(
		() =>
			campaigns.find((campaign) => campaign.id === selectedCampaignId) ??
			campaigns.find((campaign) => campaign.status === "active") ??
			campaigns[0] ??
			null,
		[campaigns, selectedCampaignId],
	);

	const detailQuery = useQuery({
		enabled: Boolean(selectedCampaign),
		queryKey: ["enterprise", "access-review-campaign", selectedCampaign?.id],
		queryFn: ({ signal }) =>
			enterpriseClient.getAccessReviewCampaign(selectedCampaign?.id ?? "", {
				signal,
			}),
	});

	const createMutation = useMutation({
		mutationFn: (input: CreateAccessReviewCampaignInput) =>
			enterpriseClient.createAccessReviewCampaign(input),
		onSuccess: async (detail) => {
			setSelectedCampaignId(detail.campaign.id);
			await queryClient.invalidateQueries({
				queryKey: enterpriseQueryKeys.accessReviewCampaigns,
			});
		},
	});

	const activeCampaigns = campaigns.filter(
		(campaign) => campaign.status === "active",
	).length;
	const pendingItems = campaigns.reduce(
		(total, campaign) => total + campaign.pending_items,
		0,
	);
	const disabled =
		createMutation.isPending || !form.name.trim() || !hasScope(form.scope);

	return (
		<div className="flex flex-col gap-6">
			<header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
				<div>
					<div className="flex items-center gap-2">
						<h1 className="text-2xl font-heading font-semibold">
							Access reviews
						</h1>
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

			{campaignsQuery.error ? (
				<Alert variant="destructive">
					<AlertTitle>Access reviews failed to load</AlertTitle>
					<AlertDescription>{campaignsQuery.error.message}</AlertDescription>
				</Alert>
			) : null}

			<section className="grid gap-4 xl:grid-cols-[360px_minmax(0,1fr)]">
				<Card className="rounded-lg" size="sm">
					<CardHeader>
						<CardTitle>New campaign</CardTitle>
					</CardHeader>
					<CardContent>
						<form
							className="flex flex-col gap-4"
							onSubmit={(event) => {
								event.preventDefault();
								createMutation.mutate({
									name: form.name,
									due_at: new Date(`${form.dueDate}T23:59:00`).toISOString(),
									scope: form.scope,
								});
							}}
						>
							<div className="grid gap-2">
								<Label htmlFor="access-review-name">Name</Label>
								<Input
									id="access-review-name"
									value={form.name}
									onChange={(event) =>
										setForm((current) => ({
											...current,
											name: event.target.value,
										}))
									}
								/>
							</div>
							<div className="grid gap-2">
								<Label htmlFor="access-review-due">Due date</Label>
								<Input
									id="access-review-due"
									type="date"
									value={form.dueDate}
									onChange={(event) =>
										setForm((current) => ({
											...current,
											dueDate: event.target.value,
										}))
									}
								/>
							</div>
							<div className="grid gap-3">
								<Label>Scope</Label>
								{scopeOptions.map(([key, label]) => (
									<label key={key} className="flex items-center gap-2 text-sm">
										<Checkbox
											checked={form.scope[key]}
											onCheckedChange={(checked) =>
												setForm((current) => ({
													...current,
													scope: { ...current.scope, [key]: checked === true },
												}))
											}
										/>
										<span>{label}</span>
									</label>
								))}
							</div>
							{createMutation.error ? (
								<p className="text-sm text-destructive">
									{createMutation.error.message}
								</p>
							) : null}
							<Button type="submit" disabled={disabled}>
								<ClipboardCheck className="size-4" />
								Start campaign
							</Button>
						</form>
					</CardContent>
				</Card>

				<Card className="rounded-lg" size="sm">
					<CardHeader className="flex flex-row items-center justify-between gap-3">
						<CardTitle>Campaign inventory</CardTitle>
						<Button
							type="button"
							variant="outline"
							size="sm"
							onClick={() => campaignsQuery.refetch()}
						>
							<RefreshCw className="size-3.5" />
							Refresh
						</Button>
					</CardHeader>
					<CardContent>
						{campaignsQuery.isPending ? (
							<Skeleton className="h-56 w-full rounded-lg" />
						) : (
							<CampaignTable
								campaigns={campaigns}
								selectedCampaignId={selectedCampaign?.id ?? null}
								onSelect={setSelectedCampaignId}
							/>
						)}
					</CardContent>
				</Card>
			</section>

			<Card className="rounded-lg" size="sm">
				<CardHeader>
					<CardTitle>Review snapshot</CardTitle>
				</CardHeader>
				<CardContent>
					{detailQuery.isPending && selectedCampaign ? (
						<Skeleton className="h-64 w-full rounded-lg" />
					) : (
						<ReviewItemsTable items={detailQuery.data?.items ?? []} />
					)}
				</CardContent>
			</Card>
		</div>
	);
}

function defaultDueDate() {
	const due = new Date();
	due.setDate(due.getDate() + 14);
	return due.toISOString().slice(0, 10);
}

function hasScope(scope: Record<ScopeKey, boolean>) {
	return scopeOptions.some(([key]) => scope[key]);
}
