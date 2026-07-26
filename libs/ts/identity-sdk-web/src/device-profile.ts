export type DevicePlatform = 'android' | 'ios' | 'linux' | 'macos' | 'windows' | 'other';
export type DeviceFormFactor = 'desktop' | 'mobile' | 'tablet' | 'other';
export type ScreenBucket = 'compact' | 'medium' | 'large' | 'wide';

export interface DeviceProfile {
  version: 1;
  platform: DevicePlatform;
  form_factor: DeviceFormFactor;
  cpu_bucket: 1 | 2 | 4 | 8 | 16;
  memory_bucket?: 1 | 2 | 4 | 8;
  touch_capable: boolean;
  color_depth_bucket: 16 | 24 | 30 | 32;
  screen_bucket: ScreenBucket;
  timezone_offset_bucket: number;
}

type NavigatorWithMemory = Navigator & { deviceMemory?: number };

export function collectDeviceProfile(): DeviceProfile {
  const touchCapable = navigator.maxTouchPoints > 0;
  const shortestSide = Math.min(screen.width, screen.height);

  return {
    version: 1,
    platform: classifyPlatform(navigator.userAgent),
    form_factor: classifyFormFactor(navigator.userAgent, touchCapable, shortestSide),
    cpu_bucket: bucketCpu(navigator.hardwareConcurrency),
    ...memoryBucket((navigator as NavigatorWithMemory).deviceMemory),
    touch_capable: touchCapable,
    color_depth_bucket: bucketColorDepth(screen.colorDepth),
    screen_bucket: bucketScreen(screen.width, screen.height),
    timezone_offset_bucket: Math.max(
      -14,
      Math.min(14, Math.round(-new Date().getTimezoneOffset() / 60)),
    ),
  };
}

export function classifyPlatform(userAgent: string): DevicePlatform {
  const value = userAgent.toLowerCase();
  if (value.includes('android')) return 'android';
  if (value.includes('iphone') || value.includes('ipad')) return 'ios';
  if (value.includes('windows')) return 'windows';
  if (value.includes('macintosh') || value.includes('mac os')) return 'macos';
  if (value.includes('linux')) return 'linux';
  return 'other';
}

export function classifyFormFactor(
  userAgent: string,
  touchCapable: boolean,
  shortestSide: number,
): DeviceFormFactor {
  const value = userAgent.toLowerCase();
  if (value.includes('ipad') || (touchCapable && shortestSide >= 600 && shortestSide < 1_100)) {
    return 'tablet';
  }
  if (value.includes('mobile') || value.includes('iphone') || value.includes('android')) {
    return 'mobile';
  }
  return shortestSide >= 600 ? 'desktop' : 'other';
}

export function bucketScreen(width: number, height: number): ScreenBucket {
  const longestSide = Math.max(width, height);
  if (longestSide < 900) return 'compact';
  if (longestSide < 1_600) return 'medium';
  if (longestSide < 2_400) return 'large';
  return 'wide';
}

function bucketCpu(value: number): 1 | 2 | 4 | 8 | 16 {
  if (value <= 1) return 1;
  if (value <= 2) return 2;
  if (value <= 4) return 4;
  if (value <= 8) return 8;
  return 16;
}

function memoryBucket(value: number | undefined): {
  memory_bucket?: 1 | 2 | 4 | 8;
} {
  if (value === undefined || !Number.isFinite(value)) return {};
  if (value <= 1) return { memory_bucket: 1 };
  if (value <= 2) return { memory_bucket: 2 };
  if (value <= 4) return { memory_bucket: 4 };
  return { memory_bucket: 8 };
}

function bucketColorDepth(value: number): 16 | 24 | 30 | 32 {
  if (value <= 16) return 16;
  if (value <= 24) return 24;
  if (value <= 30) return 30;
  return 32;
}
