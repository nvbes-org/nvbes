/**
 * Bot Guard — Environment signals collector
 *
 * Collects passive environment probes that distinguish headless/bot contexts
 * from real browser environments. All probes are async-safe and handle
 * missing APIs gracefully (returns sentinel values, never throws).
 *
 * False positive mitigations:
 * - Speech synthesis is collected only after a user interaction (voices load async)
 * - Battery API is only meaningful if UA claims to be Chrome mobile
 * - localStorage failure is treated as soft signal (private browsing)
 * - All fields are optional on the server — missing = skipped, not scored
 */

export type PermissionState = 'granted' | 'denied' | 'prompt' | 'unavailable';

export interface EnvironmentSignals {
  speech_voices_count: number;
  nav_touch_points: number;
  nav_hw_concurrency: number | null;
  nav_device_memory: number | null;
  lang: string;
  tz: string;
  tz_offset: number;
  perm_notifications: PermissionState;
  raf_variance: number;
  raf_mean: number;
  battery_available: boolean;
  storage_ok: boolean;
}

// ---------------------------------------------------------------------------
// Individual probes
// ---------------------------------------------------------------------------

function probeSpeechVoices(): number {
  // speechSynthesis.getVoices() may return [] before user interaction.
  // The caller (collectEnvironmentSignals) must be invoked after a user gesture.
  try {
    return window.speechSynthesis?.getVoices().length ?? -1;
  } catch {
    return -1;
  }
}

function probeNavigator(): {
  touch_points: number;
  hw_concurrency: number | null;
  device_memory: number | null;
} {
  return {
    touch_points: navigator.maxTouchPoints ?? 0,
    hw_concurrency: navigator.hardwareConcurrency ?? null,
    // deviceMemory is not in the standard TS lib yet
    device_memory: (navigator as Navigator & { deviceMemory?: number }).deviceMemory ?? null,
  };
}

function probeLanguageTz(): { lang: string; tz: string; tz_offset: number } {
  let tz = 'unknown';
  try {
    tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
  } catch {
    // no-op
  }
  return {
    lang: navigator.language ?? '',
    tz,
    tz_offset: new Date().getTimezoneOffset(),
  };
}

async function probePermissions(): Promise<PermissionState> {
  try {
    if (!navigator.permissions) return 'unavailable';
    const result = await navigator.permissions.query({ name: 'notifications' });
    return result.state as PermissionState;
  } catch {
    return 'unavailable';
  }
}

async function probeRaf(): Promise<{ variance: number; mean: number }> {
  return new Promise((resolve) => {
    const deltas: number[] = [];
    let prev = 0;
    let frame = 0;

    const tick = (ts: number) => {
      if (prev > 0) deltas.push(ts - prev);
      prev = ts;
      if (++frame < 10) {
        requestAnimationFrame(tick);
      } else {
        const mean = deltas.reduce((s, v) => s + v, 0) / deltas.length;
        const variance = deltas.reduce((s, v) => s + (v - mean) ** 2, 0) / deltas.length;
        resolve({ variance, mean });
      }
    };

    requestAnimationFrame(tick);
  });
}

async function probeBattery(): Promise<boolean> {
  try {
    // getBattery is non-standard — only score if UA = Chrome mobile
    const chromeMobile =
      /Chrome/.test(navigator.userAgent) && /Mobile|Android/.test(navigator.userAgent);
    if (!chromeMobile) return true; // not applicable → neutral
    return (
      typeof (navigator as Navigator & { getBattery?: () => Promise<unknown> }).getBattery ===
      'function'
    );
  } catch {
    return true; // neutral on error
  }
}

function probeStorage(): boolean {
  try {
    const key = `_bg_${Math.random().toString(36).slice(2, 8)}`;
    localStorage.setItem(key, '1');
    const ok = localStorage.getItem(key) === '1';
    localStorage.removeItem(key);
    return ok;
  } catch {
    // Private browsing mode — not a bot signal, return neutral
    return true;
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Collects all environment signals asynchronously.
 *
 * @param afterUserInteraction - Set to true once the user has clicked or typed.
 *   Speech synthesis voices are only reliably available after a user gesture.
 */
export async function collectEnvironmentSignals(
  afterUserInteraction = false,
): Promise<EnvironmentSignals> {
  const [perm, raf] = await Promise.all([probePermissions(), probeRaf()]);

  const nav = probeNavigator();
  const langTz = probeLanguageTz();
  const batteryOk = await probeBattery();
  const storageOk = probeStorage();

  // Voices are only meaningful after a user gesture.
  const voicesCount = afterUserInteraction ? probeSpeechVoices() : -1;

  return {
    speech_voices_count: voicesCount,
    nav_touch_points: nav.touch_points,
    nav_hw_concurrency: nav.hw_concurrency,
    nav_device_memory: nav.device_memory,
    lang: langTz.lang,
    tz: langTz.tz,
    tz_offset: langTz.tz_offset,
    perm_notifications: perm,
    raf_variance: raf.variance,
    raf_mean: raf.mean,
    battery_available: batteryOk,
    storage_ok: storageOk,
  };
}
