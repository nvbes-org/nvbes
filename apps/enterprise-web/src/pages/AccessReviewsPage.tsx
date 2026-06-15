import type {
	CloseAccessReviewCampaignInput,
	AccessReviewDecisionInput,
	CreateAccessReviewCampaignInput,
} from "@nvbes/identity-client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import { useMemo, useState } from "react";
import { Alert, AlertDescription, AlertTitle } from "../components/ui/alert";
import { Button } from "../components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "../components/ui/card";
import { Skeleton } from "../components/ui/skeleton";
import { enterpriseClient } from "../enterprise.api";
import {
	accessReviewCampaignsQueryOptions,
	accessReviewSchedulesQueryOptions,
	enterpriseQueryKeys,
} from "../enterprise.queries";
import { AccessReviewCloseControls } from "./AccessReviewsPage.closeControls";
import { CreateAccessReviewCampaignForm } from "./AccessReviewsPage.create";
import { downloadAccessReviewExport } from "./AccessReviewsPage.export";
import { AccessReviewExportControls } from "./AccessReviewsPage.exportControls";
import {
	type AccessReviewFilters,
	filterAccessReviewItems,
} from "./AccessReviewsPage.filters";
import { AccessReviewsPageHeader } from "./AccessReviewsPage.header";
import { accessReviewCampaignQueryKey } from "./AccessReviewsPage.keys";
import { useAccessReviewScheduleMutations } from "./AccessReviewsPage.scheduleMutations";
import {
	type ChangeTargetRole,
	AccessReviewDecisionControls,
} from "./AccessReviewsPage.reviewControls";
import { AccessReviewSchedulesPanel } from "./AccessReviewsPage.schedules";
import { CampaignTable, ReviewItemsTable } from "./AccessReviewsPage.tables";

export function AccessReviewsPage() {
	const queryClient = useQueryClient();
	const campaignsQuery = useQuery(accessReviewCampaignsQueryOptions());
	const schedulesQuery = useQuery(accessReviewSchedulesQueryOptions());
	const [selectedCampaignId, setSelectedCampaignId] = useState<string | null>(
		null,
	);
	const [reviewNote, setReviewNote] = useState("");
	const [targetRole, setTargetRole] = useState<ChangeTargetRole>("viewer");
	const [filters, setFilters] = useState<AccessReviewFilters>({
		itemType: "all",
		decision: "pending",
	});

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
		queryKey: accessReviewCampaignQueryKey(selectedCampaign?.id ?? null),
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

	const {
		createScheduleMutation,
		runScheduleMutation,
		updateScheduleMutation,
	} = useAccessReviewScheduleMutations({
		onRunNowSuccess: setSelectedCampaignId,
	});

	const decisionMutation = useMutation({
		mutationFn: ({
			itemId,
			input,
		}: {
			itemId: string;
			input: AccessReviewDecisionInput;
		}) => {
			if (!selectedCampaign) {
				throw new Error("No access review campaign selected.");
			}
			return enterpriseClient.decideAccessReviewItem(
				selectedCampaign.id,
				itemId,
				input,
			);
		},
		onSuccess: async (result) => {
			await Promise.all([
				queryClient.invalidateQueries({
					queryKey: enterpriseQueryKeys.accessReviewCampaigns,
				}),
				queryClient.invalidateQueries({
					queryKey: accessReviewCampaignQueryKey(result.campaign.id),
				}),
			]);
		},
	});

	const exportMutation = useMutation({
		mutationFn: async (format: "csv" | "json") => {
			if (!selectedCampaign) {
				throw new Error("No access review campaign selected.");
			}
			const exportData = await enterpriseClient.exportAccessReviewCampaign(
				selectedCampaign.id,
			);
			return { exportData, format };
		},
		onSuccess: ({ exportData, format }) => {
			downloadAccessReviewExport(exportData, format);
		},
	});

	const closeMutation = useMutation({
		mutationFn: async ({
			campaignId,
			input,
		}: {
			campaignId: string;
			input: CloseAccessReviewCampaignInput;
		}) => enterpriseClient.closeAccessReviewCampaign(campaignId, input),
		onSuccess: async (detail) => {
			setSelectedCampaignId(detail.campaign.id);
			await Promise.all([
				queryClient.invalidateQueries({
					queryKey: enterpriseQueryKeys.accessReviewCampaigns,
				}),
				queryClient.invalidateQueries({
					queryKey: accessReviewCampaignQueryKey(detail.campaign.id),
				}),
			]);
		},
	});

	const activeCampaigns = campaigns.filter(
		(campaign) => campaign.status === "active",
	).length;
	const pendingItems = campaigns.reduce(
		(total, campaign) => total + campaign.pending_items,
		0,
	);
	const reviewItems = detailQuery.data?.items ?? [];
	const filteredReviewItems = filterAccessReviewItems(reviewItems, filters);
	return (
		<div className="flex flex-col gap-6">
			<AccessReviewsPageHeader
				activeCampaigns={activeCampaigns}
				pendingItems={pendingItems}
			/>

			{campaignsQuery.error ? (
				<Alert variant="destructive">
					<AlertTitle>Access reviews failed to load</AlertTitle>
					<AlertDescription>{campaignsQuery.error.message}</AlertDescription>
				</Alert>
			) : null}

			<AccessReviewSchedulesPanel
				error={schedulesQuery.error}
				createError={createScheduleMutation.error}
				pending={schedulesQuery.isPending}
				createPending={createScheduleMutation.isPending}
				mutatingScheduleId={
					updateScheduleMutation.variables?.scheduleId ??
					runScheduleMutation.variables ??
					null
				}
				schedules={schedulesQuery.data?.schedules ?? []}
				runError={runScheduleMutation.error}
				updateError={updateScheduleMutation.error}
				onRefresh={() => schedulesQuery.refetch()}
				onCreate={createScheduleMutation.mutate}
				onDisable={(scheduleId) =>
					updateScheduleMutation.mutate({ action: "disable", scheduleId })
				}
				onEnable={(scheduleId) =>
					updateScheduleMutation.mutate({ action: "enable", scheduleId })
				}
				onRunNow={runScheduleMutation.mutate}
			/>

			<section className="grid gap-4 xl:grid-cols-[360px_minmax(0,1fr)]">
				<Card className="rounded-lg" size="sm">
					<CardHeader>
						<CardTitle>New campaign</CardTitle>
					</CardHeader>
					<CardContent>
						<CreateAccessReviewCampaignForm
							error={createMutation.error}
							pending={createMutation.isPending}
							onSubmit={createMutation.mutate}
						/>
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
					<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
						<CardTitle>Review snapshot</CardTitle>
						<AccessReviewExportControls
							disabled={!selectedCampaign || exportMutation.isPending}
							onExport={exportMutation.mutate}
						/>
					</div>
				</CardHeader>
				<CardContent>
					{exportMutation.error ? (
						<Alert variant="destructive" className="mb-4">
							<AlertTitle>Export failed</AlertTitle>
							<AlertDescription>{exportMutation.error.message}</AlertDescription>
						</Alert>
					) : null}
					{decisionMutation.error ? (
						<Alert variant="destructive" className="mb-4">
							<AlertTitle>Decision failed</AlertTitle>
							<AlertDescription>
								{decisionMutation.error.message}
							</AlertDescription>
						</Alert>
					) : null}
					{closeMutation.error ? (
						<Alert variant="destructive" className="mb-4">
							<AlertTitle>Campaign close failed</AlertTitle>
							<AlertDescription>{closeMutation.error.message}</AlertDescription>
						</Alert>
					) : null}
					<AccessReviewDecisionControls
						reviewNote={reviewNote}
						targetRole={targetRole}
						filters={filters}
						onReviewNoteChange={setReviewNote}
						onTargetRoleChange={setTargetRole}
						onFiltersChange={setFilters}
					/>
					<div className="mb-4 flex justify-end">
						<AccessReviewCloseControls
							campaign={selectedCampaign}
							pending={closeMutation.isPending}
							onClose={(campaignId, input) =>
								closeMutation.mutate({ campaignId, input })
							}
						/>
					</div>
					{detailQuery.isPending && selectedCampaign ? (
						<Skeleton className="h-64 w-full rounded-lg" />
					) : (
						<ReviewItemsTable
							items={filteredReviewItems}
							disabled={
								decisionMutation.isPending ||
								detailQuery.data?.campaign.status === "closed"
							}
							noteReady={reviewNote.trim().length > 0}
							targetRole={targetRole}
							onDecide={(itemId, decision) =>
								decisionMutation.mutate({
									itemId,
									input: {
										decision,
										note:
											decision === "approved" || !reviewNote.trim()
												? undefined
												: reviewNote.trim(),
										change:
											decision === "changed"
												? { target_role: targetRole }
												: undefined,
									},
								})
							}
						/>
					)}
				</CardContent>
			</Card>
		</div>
	);
}
