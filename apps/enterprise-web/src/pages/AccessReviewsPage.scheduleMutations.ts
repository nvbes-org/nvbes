import type { CreateAccessReviewScheduleInput } from "@nvbes/identity-client";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { enterpriseClient } from "../enterprise.api";
import { enterpriseQueryKeys } from "../enterprise.queries";

export function useAccessReviewScheduleMutations({
	onRunNowSuccess,
}: {
	onRunNowSuccess: (campaignId: string) => void;
}) {
	const queryClient = useQueryClient();

	const createScheduleMutation = useMutation({
		mutationFn: (input: CreateAccessReviewScheduleInput) =>
			enterpriseClient.createAccessReviewSchedule(input),
		onSuccess: async () => {
			await queryClient.invalidateQueries({
				queryKey: enterpriseQueryKeys.accessReviewSchedules,
			});
		},
	});

	const updateScheduleMutation = useMutation({
		mutationFn: ({
			action,
			scheduleId,
		}: {
			action: "disable" | "enable";
			scheduleId: string;
		}) =>
			action === "disable"
				? enterpriseClient.disableAccessReviewSchedule(scheduleId)
				: enterpriseClient.enableAccessReviewSchedule(scheduleId),
		onSuccess: async () => {
			await queryClient.invalidateQueries({
				queryKey: enterpriseQueryKeys.accessReviewSchedules,
			});
		},
	});

	const runScheduleMutation = useMutation({
		mutationFn: (scheduleId: string) =>
			enterpriseClient.runAccessReviewScheduleNow(scheduleId),
		onSuccess: async (detail) => {
			onRunNowSuccess(detail.campaign.id);
			await Promise.all([
				queryClient.invalidateQueries({
					queryKey: enterpriseQueryKeys.accessReviewSchedules,
				}),
				queryClient.invalidateQueries({
					queryKey: enterpriseQueryKeys.accessReviewCampaigns,
				}),
			]);
		},
	});

	return {
		createScheduleMutation,
		runScheduleMutation,
		updateScheduleMutation,
	};
}
