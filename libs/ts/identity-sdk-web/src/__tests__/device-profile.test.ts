import { describe, expect, it } from 'vite-plus/test';

import { bucketScreen, classifyFormFactor, classifyPlatform } from '../device-profile';

describe('device profile minimization', () => {
  it('reduces platform strings to a small allowlist', () => {
    expect(classifyPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X)')).toBe('macos');
    expect(classifyPlatform('custom-client')).toBe('other');
  });

  it('uses coarse screen buckets instead of exact dimensions', () => {
    expect(bucketScreen(390, 844)).toBe('compact');
    expect(bucketScreen(1_440, 900)).toBe('medium');
    expect(bucketScreen(3_840, 2_160)).toBe('wide');
  });

  it('distinguishes mobile and tablet without exposing a model', () => {
    expect(classifyFormFactor('Mozilla/5.0 iPhone Mobile', true, 390)).toBe('mobile');
    expect(classifyFormFactor('Mozilla/5.0 iPad', true, 820)).toBe('tablet');
  });
});
