import { getSafeLocalStorage } from './safe-storage';

export interface VersionMismatchInput {
  frontendBuildId?: string | null;
  backendReleaseId?: string | null;
}

export interface VersionMismatchDetectorOptions extends VersionMismatchInput {
  cooldownMs?: number;
  storageKey?: string;
  now?: () => number;
}

export interface VersionMismatchResult {
  mismatched: boolean;
  shouldPrompt: boolean;
  frontendBuildId: string | null;
  backendReleaseId: string | null;
}

const DEFAULT_STORAGE_KEY = 'nvbes.version-mismatch.lastPromptAt';
const DEFAULT_COOLDOWN_MS = 5 * 60 * 1000;

export function detectVersionMismatch(input: VersionMismatchInput): boolean {
  const frontendBuildId = normalizeReleaseId(input.frontendBuildId);
  const backendReleaseId = normalizeReleaseId(input.backendReleaseId);
  return Boolean(frontendBuildId && backendReleaseId && frontendBuildId !== backendReleaseId);
}

export function inspectVersionMismatch(
  options: VersionMismatchDetectorOptions,
): VersionMismatchResult {
  const frontendBuildId = normalizeReleaseId(options.frontendBuildId);
  const backendReleaseId = normalizeReleaseId(options.backendReleaseId);
  const mismatched = detectVersionMismatch({ frontendBuildId, backendReleaseId });

  return {
    backendReleaseId,
    frontendBuildId,
    mismatched,
    shouldPrompt:
      mismatched &&
      canPromptVersionMismatch({
        cooldownMs: options.cooldownMs,
        now: options.now,
        storageKey: options.storageKey,
      }),
  };
}

export function recordVersionMismatchPrompt(options: VersionMismatchDetectorOptions = {}): void {
  getSafeLocalStorage().setItem(
    options.storageKey ?? DEFAULT_STORAGE_KEY,
    String((options.now ?? Date.now)()),
  );
}

function canPromptVersionMismatch(
  options: Pick<VersionMismatchDetectorOptions, 'cooldownMs' | 'now' | 'storageKey'>,
): boolean {
  const storage = getSafeLocalStorage();
  const raw = storage.getItem(options.storageKey ?? DEFAULT_STORAGE_KEY);
  const lastPromptAt = raw ? Number(raw) : Number.NaN;
  if (!Number.isFinite(lastPromptAt)) {
    return true;
  }

  return (options.now ?? Date.now)() - lastPromptAt >= (options.cooldownMs ?? DEFAULT_COOLDOWN_MS);
}

function normalizeReleaseId(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  return normalized ? normalized : null;
}
