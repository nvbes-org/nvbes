import { useEffect, useState } from "react";
import { z } from "zod";
import { useAccountContext } from "@/hooks/useAccountContext";
import { identityHttpClient } from "../identity.http";
import type { WorkerQueueStatusResponse } from "./WorkerQueueStatusPage.types";

const WorkerQueueStatusSchema = z.object({
	workspace_id: z.string(),
	queue_name: z.string(),
	snapshot_at: z.string(),
	statuses: z.array(
		z.object({
			status: z.string(),
			depth: z.number(),
			oldest_age_seconds: z.number().nullable(),
		}),
	),
});

export function useWorkerQueueStatusPage() {
	const { me, loading: accountLoading } = useAccountContext();
	const [submittedWorkspaceId, setSubmittedWorkspaceId] = useState("");
	const [snapshot, setSnapshot] = useState<WorkerQueueStatusResponse | null>(
		null,
	);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState("");

	useEffect(() => {
		const workspaceId = submittedWorkspaceId || me?.current_workspace_id || "";
		if (!workspaceId) {
			return;
		}

		const controller = new AbortController();

		const fetchSnapshot = async () => {
			setLoading(true);
			setError("");

			try {
				const data = await identityHttpClient.get(
					`/workspaces/${workspaceId}/worker-queue/status`,
					WorkerQueueStatusSchema,
					{
						signal: controller.signal,
					},
				);

				setSnapshot({
					workspace_id: data.workspace_id,
					queue_name: data.queue_name,
					snapshot_at: data.snapshot_at,
					statuses: data.statuses,
				});
			} catch (fetchError) {
				if (
					fetchError instanceof DOMException &&
					fetchError.name === "AbortError"
				) {
					return;
				}
				setError("Impossible de charger la supervision du worker.");
				setSnapshot(null);
			} finally {
				setLoading(false);
			}
		};

		void fetchSnapshot();
		return () => controller.abort();
	}, [me?.current_workspace_id, submittedWorkspaceId]);

	return {
		me,
		accountLoading,
		submittedWorkspaceId,
		setSubmittedWorkspaceId,
		snapshot,
		loading,
		error,
	};
}
