export type ReplayableDelivery = {
  status: 'pending' | 'delivered' | 'failed' | 'replayed';
  attempt_count: number;
};

export function canReplayWebhookDelivery(delivery: ReplayableDelivery): boolean {
  return delivery.status === 'failed' || delivery.status === 'pending';
}
