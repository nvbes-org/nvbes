import type { AccountSession } from '@nvbes/identity-client';

export type OsKey = 'macos' | 'windows' | 'linux' | 'android' | 'ios' | 'unknown';
export type BrowserKey = 'chrome' | 'firefox' | 'edge' | 'safari' | 'opera' | 'brave' | 'unknown';
export type DeviceType = 'desktop' | 'mobile' | 'tablet';

export interface ParsedDevice {
  browser: string;
  browserKey: BrowserKey;
  os: string;
  osKey: OsKey;
  deviceType: DeviceType;
}

export function parseUserAgent(ua: string): ParsedDevice {
  let browser = 'Navigateur inconnu';
  let browserKey: BrowserKey = 'unknown';
  let os = 'OS inconnu';
  let osKey: OsKey = 'unknown';
  let deviceType: DeviceType = 'desktop';

  // Browser detection (order matters)
  if (ua.includes('Brave')) {
    browser = 'Brave';
    browserKey = 'brave';
  } else if (ua.includes('OPR') || ua.includes('Opera')) {
    browser = 'Opera';
    browserKey = 'opera';
  } else if (ua.includes('Edg')) {
    browser = 'Edge';
    browserKey = 'edge';
  } else if (ua.includes('Firefox')) {
    browser = 'Firefox';
    browserKey = 'firefox';
  } else if (ua.includes('Chrome')) {
    browser = 'Chrome';
    browserKey = 'chrome';
  } else if (ua.includes('Safari')) {
    browser = 'Safari';
    browserKey = 'safari';
  }

  // OS detection
  if (ua.includes('Android')) {
    os = 'Android';
    osKey = 'android';
    deviceType = ua.includes('Tablet') ? 'tablet' : 'mobile';
  } else if (ua.includes('iPhone')) {
    os = 'iOS';
    osKey = 'ios';
    deviceType = 'mobile';
  } else if (ua.includes('iPad')) {
    os = 'iPadOS';
    osKey = 'ios';
    deviceType = 'tablet';
  } else if (ua.includes('Mac OS') || ua.includes('Macintosh')) {
    os = 'macOS';
    osKey = 'macos';
  } else if (ua.includes('Windows')) {
    os = 'Windows';
    osKey = 'windows';
  } else if (ua.includes('Linux')) {
    os = 'Linux';
    osKey = 'linux';
  }

  return { browser, browserKey, os, osKey, deviceType };
}

export function trustScoreColor(score: number | null): string {
  if (score === null) return 'text-muted-foreground';
  if (score >= 70) return 'text-emerald-600';
  if (score >= 40) return 'text-amber-500';
  return 'text-red-500';
}

export function trustProgressColor(score: number | null): string {
  if (score === null) return 'bg-muted';
  if (score >= 70) return 'bg-emerald-500';
  if (score >= 40) return 'bg-amber-400';
  return 'bg-red-500';
}

export function riskLabel(decision: string | null): string {
  if (decision === 'allow') return 'Autorisé';
  if (decision === 'challenge') return 'Challengé';
  if (decision === 'deny') return 'Refusé';
  return 'Inconnu';
}

export interface DeviceGroup {
  id: string;
  deviceId: string | null;
  userAgent: string | null;
  parsedDevice: ParsedDevice;
  isCurrentDevice: boolean;
  trustLevel: string | null;
  trustScore: number | null;
  isRecognized: boolean;
  sessions: AccountSession[];
  lastSeenAt: string;
}

export function groupSessionsByDevice(sessions: AccountSession[]): {
  currentDevice: DeviceGroup | null;
  recognizedDevices: DeviceGroup[];
  otherDevices: DeviceGroup[];
} {
  const map = new Map<string, AccountSession[]>();

  for (const session of sessions) {
    const key = session.device_id || session.user_agent || 'unknown-device';
    const group = map.get(key) ?? [];
    group.push(session);
    map.set(key, group);
  }

  const deviceGroups: DeviceGroup[] = [];

  for (const [key, deviceSessions] of map.entries()) {
    const sorted = [...deviceSessions].sort((a, b) => {
      if (a.current) return -1;
      if (b.current) return 1;
      return new Date(b.last_seen_at).getTime() - new Date(a.last_seen_at).getTime();
    });

    const primary = sorted[0];
    const userAgent = primary.user_agent;
    const parsed = userAgent
      ? parseUserAgent(userAgent)
      : {
          browser: 'Appareil inconnu',
          browserKey: 'unknown' as const,
          os: '',
          osKey: 'unknown' as const,
          deviceType: 'desktop' as const,
        };

    const isCurrentDevice = sorted.some((s) => s.current);

    let trustLevel: string | null = null;
    if (sorted.some((s) => s.device_trust_level === 'trusted')) {
      trustLevel = 'trusted';
    } else if (sorted.some((s) => s.device_trust_level === 'recognized')) {
      trustLevel = 'recognized';
    } else if (sorted.some((s) => s.device_trust_level === 'restricted')) {
      trustLevel = 'restricted';
    } else {
      trustLevel = primary.device_trust_level;
    }

    const isRecognized = trustLevel === 'trusted' || trustLevel === 'recognized';

    const scores = sorted
      .map((s) => s.device_trust_score)
      .filter((score): score is number => score !== null);
    const trustScore = scores.length > 0 ? Math.max(...scores) : primary.device_trust_score;

    const lastSeenAt = sorted.reduce(
      (latest, s) =>
        new Date(s.last_seen_at).getTime() > new Date(latest).getTime() ? s.last_seen_at : latest,
      primary.last_seen_at,
    );

    deviceGroups.push({
      id: key,
      deviceId: primary.device_id,
      userAgent,
      parsedDevice: parsed,
      isCurrentDevice,
      trustLevel,
      trustScore,
      isRecognized,
      sessions: sorted,
      lastSeenAt,
    });
  }

  deviceGroups.sort((a, b) => new Date(b.lastSeenAt).getTime() - new Date(a.lastSeenAt).getTime());

  const currentDevice = deviceGroups.find((d) => d.isCurrentDevice) ?? null;
  const nonCurrentDevices = deviceGroups.filter((d) => !d.isCurrentDevice);

  const recognizedDevices = nonCurrentDevices.filter((d) => d.isRecognized);
  const otherDevices = nonCurrentDevices.filter((d) => !d.isRecognized);

  return { currentDevice, recognizedDevices, otherDevices };
}
