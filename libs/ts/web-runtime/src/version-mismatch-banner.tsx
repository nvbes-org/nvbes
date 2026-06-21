import { useEffect, useState } from 'react';
import { z } from 'zod';
import {
  inspectVersionMismatch,
  recordVersionMismatchPrompt,
  type VersionMismatchResult,
} from './version-mismatch';
import { verifiedFetchJson } from './verified-fetch';

const HealthReleaseSchema = z.object({
  release_id: z.string().optional().nullable(),
});

export interface VersionMismatchBannerProps {
  frontendBuildId: string | null | undefined;
  healthUrl: string;
  appName: string;
}

export function VersionMismatchBanner({
  frontendBuildId,
  healthUrl,
  appName,
}: VersionMismatchBannerProps) {
  const [result, setResult] = useState<VersionMismatchResult | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function checkVersion(): Promise<void> {
      try {
        const health = await verifiedFetchJson(healthUrl, HealthReleaseSchema, {
          allowedOrigins: [healthUrl],
          cache: 'no-store',
        });
        if (cancelled) {
          return;
        }
        const mismatch = inspectVersionMismatch({
          backendReleaseId: health.release_id,
          frontendBuildId,
          storageKey: `nvbes.version-mismatch.${appName}.lastPromptAt`,
        });
        if (mismatch.shouldPrompt) {
          recordVersionMismatchPrompt({
            storageKey: `nvbes.version-mismatch.${appName}.lastPromptAt`,
          });
          setResult(mismatch);
        }
      } catch {
        // Version checks must never block the app shell.
      }
    }

    void checkVersion();

    return () => {
      cancelled = true;
    };
  }, [appName, frontendBuildId, healthUrl]);

  if (!result?.shouldPrompt) {
    return null;
  }

  return (
    <aside className="fixed right-4 bottom-4 z-[10000] w-[min(24rem,calc(100vw-2rem))] rounded-md border border-amber-500/30 bg-background p-4 text-sm shadow-lg">
      <h2 className="font-semibold text-foreground">Nouvelle version disponible</h2>
      <p className="mt-1 text-muted-foreground">
        L'application et l'API ne sont plus sur la même release. Rechargez pour éviter des erreurs
        après déploiement.
      </p>
      <button
        type="button"
        className="mt-3 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground"
        onClick={() => window.location.reload()}
      >
        Recharger
      </button>
    </aside>
  );
}
