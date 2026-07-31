import type { PowChallengeProof } from './identity.auth.pow';

const DEFAULT_MAX_AGE_MS = 90_000;

type PowChallengeResolver = (baseUrl: string) => Promise<PowChallengeProof>;

interface PowChallengePreloaderOptions {
  maxAgeMs?: number;
  now?: () => number;
}

interface PrefetchedPowChallenge {
  baseUrl: string;
  inUse: boolean;
  promise: Promise<PowChallengeProof>;
  startedAt: number;
}

export interface PowChallengePreloader {
  consume(baseUrl: string): Promise<PowChallengeProof>;
  prefetch(baseUrl: string): void;
}

export function createPowChallengePreloader(
  resolve: PowChallengeResolver,
  options: PowChallengePreloaderOptions = {},
): PowChallengePreloader {
  const maxAgeMs = options.maxAgeMs ?? DEFAULT_MAX_AGE_MS;
  const now = options.now ?? Date.now;
  let prefetched: PrefetchedPowChallenge | null = null;

  function readReusable(baseUrl: string): PrefetchedPowChallenge | null {
    if (
      !prefetched ||
      prefetched.baseUrl !== baseUrl ||
      prefetched.inUse ||
      now() - prefetched.startedAt >= maxAgeMs
    ) {
      return null;
    }

    return prefetched;
  }

  function start(baseUrl: string): PrefetchedPowChallenge {
    const entry: PrefetchedPowChallenge = {
      baseUrl,
      inUse: false,
      promise: resolve(baseUrl),
      startedAt: now(),
    };
    prefetched = entry;
    void entry.promise.catch(() => {
      if (prefetched === entry && !entry.inUse) {
        prefetched = null;
      }
    });
    return entry;
  }

  return {
    async consume(baseUrl) {
      const entry = readReusable(baseUrl) ?? start(baseUrl);
      entry.inUse = true;

      try {
        return await entry.promise;
      } finally {
        if (prefetched === entry) {
          prefetched = null;
        }
      }
    },
    prefetch(baseUrl) {
      if (prefetched?.baseUrl === baseUrl && prefetched.inUse) {
        return;
      }
      if (!readReusable(baseUrl)) {
        start(baseUrl);
      }
    },
  };
}
