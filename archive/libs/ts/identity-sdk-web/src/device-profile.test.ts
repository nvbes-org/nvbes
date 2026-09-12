import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import {
  bucketScreen,
  classifyFormFactor,
  classifyPlatform,
  collectDeviceProfile,
} from './device-profile';

beforeEach(() => {
  vi.stubGlobal('navigator', { userAgent: 'unknown', hardwareConcurrency: 4, maxTouchPoints: 0 });
  vi.stubGlobal('screen', { width: 1920, height: 1080, colorDepth: 24 });
  vi.spyOn(Date.prototype, 'getTimezoneOffset').mockReturnValue(-60);
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it('collects only coarse buckets and omits unavailable memory', () => {
  expect(collectDeviceProfile()).toEqual({
    version: 1,
    platform: 'other',
    form_factor: 'desktop',
    cpu_bucket: 4,
    touch_capable: false,
    color_depth_bucket: 24,
    screen_bucket: 'large',
    timezone_offset_bucket: 1,
  });
});

it.each([
  [1, 1],
  [2, 2],
  [3, 4],
  [4, 4],
  [5, 8],
  [8, 8],
  [9, 16],
  [32, 16],
])('buckets %s logical processors as %s', (cores, bucket) => {
  vi.stubGlobal('navigator', { userAgent: '', hardwareConcurrency: cores, maxTouchPoints: 1 });
  expect(collectDeviceProfile()).toMatchObject({ cpu_bucket: bucket, touch_capable: true });
});

it.each([
  [0.5, 1],
  [1, 1],
  [2, 2],
  [3, 4],
  [4, 4],
  [5, 8],
  [16, 8],
])('buckets %s GiB memory as %s', (memory, bucket) => {
  vi.stubGlobal('navigator', {
    userAgent: '',
    hardwareConcurrency: 4,
    maxTouchPoints: 0,
    deviceMemory: memory,
  });
  expect(collectDeviceProfile().memory_bucket).toBe(bucket);
});
it.each([NaN, Infinity, -Infinity])('omits nonfinite memory %s', (memory) => {
  vi.stubGlobal('navigator', {
    userAgent: '',
    hardwareConcurrency: 4,
    maxTouchPoints: 0,
    deviceMemory: memory,
  });
  expect(collectDeviceProfile()).not.toHaveProperty('memory_bucket');
});

it.each([
  [16, 16],
  [17, 24],
  [24, 24],
  [25, 30],
  [30, 30],
  [31, 32],
  [32, 32],
])('buckets color depth %s as %s', (depth, bucket) => {
  vi.stubGlobal('screen', { width: 800, height: 600, colorDepth: depth });
  expect(collectDeviceProfile().color_depth_bucket).toBe(bucket);
});

it.each([
  [899, 'compact'],
  [900, 'medium'],
  [1599, 'medium'],
  [1600, 'large'],
  [2399, 'large'],
  [2400, 'wide'],
] as const)('uses the longest screen side at %s pixels', (longest, bucket) => {
  expect(bucketScreen(400, longest)).toBe(bucket);
  expect(bucketScreen(longest, 400)).toBe(bucket);
});

it.each([
  [-900, 14],
  [-840, 14],
  [-345, 6],
  [60, -1],
  [840, -14],
  [900, -14],
])('rounds and clamps timezone offset %s to %s', (offset, bucket) => {
  vi.spyOn(Date.prototype, 'getTimezoneOffset').mockReturnValue(offset);
  expect(collectDeviceProfile().timezone_offset_bucket).toBe(bucket);
});

it.each([
  ['Mozilla/5.0 (Linux; Android 13; Pixel 7) Mobile', 'android'],
  ['Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)', 'ios'],
  ['Mozilla/5.0 (Windows NT 10.0; Win64; x64)', 'windows'],
  ['Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)', 'macos'],
  ['Mozilla/5.0 (X11; Linux x86_64)', 'linux'],
  ['unrecognized', 'other'],
])('classifies platform %s', (ua, expected) => {
  expect(classifyPlatform(ua)).toBe(expected);
});

it.each([
  ['iPad', false, 400, 'tablet'],
  ['iPhone', false, 1200, 'mobile'],
  ['PlayStation 5', true, 800, 'other'],
  ['', true, 599, 'other'],
  ['', true, 600, 'tablet'],
  ['', true, 1099, 'tablet'],
  ['', true, 1100, 'desktop'],
  ['', false, 600, 'desktop'],
] as const)('classifies form factor %s / touch=%s / side=%s', (ua, touch, side, expected) => {
  expect(classifyFormFactor(ua, touch, side)).toBe(expected);
});
