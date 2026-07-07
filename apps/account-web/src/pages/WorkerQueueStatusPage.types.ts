export type WorkerQueueStatusView = {
  status: string;
  depth: number;
  oldest_age_seconds: number | null;
};

export type WorkerQueueStatusResponse = {
  workspace_id: string;
  queue_name: string;
  snapshot_at: string;
  statuses: WorkerQueueStatusView[];
};
