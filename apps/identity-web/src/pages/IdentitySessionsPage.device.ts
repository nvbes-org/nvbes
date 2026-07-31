import type { AccountSession, AccountSessionClient } from '@nvbes/identity-client';

export type OsKey = 'macos' | 'windows' | 'linux' | 'android' | 'ios' | 'unknown';
export type BrowserKey = 'chrome' | 'firefox' | 'edge' | 'safari' | 'opera' | 'brave' | 'unknown';
export type DeviceType = 'console' | 'desktop' | 'mobile' | 'tablet' | 'unknown' | 'wearable';

export interface ParsedDevice {
  browser: string;
  browserVersion: number | null;
  browserKey: BrowserKey;
  os: string;
  osVersion: string | null;
  osKey: OsKey;
  device: string | null;
  deviceType: DeviceType;
}

export function parseSessionClient(client: AccountSessionClient | null): ParsedDevice {
  return {
    browser: client?.browser ?? 'Navigateur inconnu',
    browserVersion: client?.browser_version ?? null,
    browserKey: browserKey(client?.browser),
    os: client?.os ?? 'OS inconnu',
    osVersion: client?.os_version ?? null,
    osKey: osKey(client?.os),
    device: client?.device ?? null,
    deviceType: client?.device_type ?? 'unknown',
  };
}

function browserKey(browser: string | null | undefined): BrowserKey {
  const value = browser?.toLowerCase() ?? '';
  if (value.includes('brave')) return 'brave';
  if (value.includes('opera')) return 'opera';
  if (value.includes('edge')) return 'edge';
  if (value.includes('firefox')) return 'firefox';
  if (value.includes('chrome')) return 'chrome';
  if (value.includes('safari')) return 'safari';
  return 'unknown';
}

function osKey(os: string | null | undefined): OsKey {
  const value = os?.toLowerCase() ?? '';
  if (value.includes('android')) return 'android';
  if (value.includes('ios') || value.includes('watchos')) return 'ios';
  if (value.includes('mac')) return 'macos';
  if (value.includes('windows')) return 'windows';
  if (value.includes('linux') || value.includes('chrome os')) return 'linux';
  return 'unknown';
}

export function riskLabel(decision: string | null): string {
  if (decision === 'allow') return 'Autorisé';
  if (decision === 'challenge' || decision === 'step_up') return 'Vérification requise';
  if (decision === 'deny') return 'Refusé';
  return 'Inconnu';
}

export function trustLabel(level: string | null): string {
  if (level === 'trusted') return 'Fiable';
  if (level === 'recognized') return 'Reconnu';
  if (level === 'restricted') return 'Restreint';
  return 'Non vérifié';
}

export function formatActiveSessionCount(count: number): string {
  return count === 1 ? '1 session active' : `${count} sessions actives`;
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
    const key = session.device_id || clientFamilyKey(session) || 'unknown-device';
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
    const parsed = parseSessionClient(primary.client);

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

function clientFamilyKey(session: AccountSession): string | null {
  if (!session.client) return session.user_agent;
  return [
    session.client.browser ?? 'unknown-browser',
    session.client.os ?? 'unknown-os',
    session.client.device ?? session.client.device_type,
  ].join(':');
}
