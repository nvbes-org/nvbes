import { describe, expect, test } from 'vite-plus/test';
import { canReplayWebhookDelivery } from '../pages/WebhooksPage.helpers';

describe('webhook replay eligibility', () => {
  test('allows failed deliveries', () => {
    expect(canReplayWebhookDelivery({ status: 'failed', attempt_count: 2 })).toBe(true);
  });

  test('blocks delivered deliveries', () => {
    expect(canReplayWebhookDelivery({ status: 'delivered', attempt_count: 1 })).toBe(false);
  });
});
