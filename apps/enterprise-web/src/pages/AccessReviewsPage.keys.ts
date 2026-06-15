export function accessReviewCampaignQueryKey(campaignId: string | null) {
	return ["enterprise", "access-review-campaign", campaignId] as const;
}
