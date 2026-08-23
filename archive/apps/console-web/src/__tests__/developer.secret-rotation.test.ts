import { describe, expect, test } from 'vite-plus/test';
import { buildSecretRotationPayload } from '../pages/SecretsPage.helpers';

describe('secret rotation helper', () => {
  test('requires an overlap window of at least one hour', () => {
    expect(() => buildSecretRotationPayload({ overlapHours: 0 })).toThrow(
      'Overlap must be at least 1 hour',
    );
  });

  test('builds payload with explicit overlap hours', () => {
    expect(buildSecretRotationPayload({ overlapHours: 24 })).toEqual({ overlap_hours: 24 });
  });
});
