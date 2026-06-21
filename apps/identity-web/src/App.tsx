import { createQueryClient, ErrorBoundary, VersionMismatchBanner } from '@nvbes/web-runtime';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { Profiler, useState } from 'react';
import { router } from './identity.router';
import { TrackingConsentBanner } from './TrackingConsentBanner';
import { ApiConnectionOverlay } from './components/ApiConnectionOverlay';
import { ToastProvider } from './components/ui/toast';

const IDENTITY_WEB_BUILD_ID = import.meta.env.VITE_NVBES_BUILD_ID || '0.1.0';
const IDENTITY_HEALTH_URL = `${import.meta.env.VITE_IDENTITY_API_BASE_URL || 'http://localhost:4000'}/health`;

function handleRenderProfiler(
  id: string,
  phase: 'mount' | 'update' | 'nested-update',
  actualDuration: number,
  baseDuration: number,
  startTime: number,
  commitTime: number,
) {
  if (actualDuration < 8) {
    return;
  }

  // eslint-disable-next-line no-console
  console.debug('[Profiler]', {
    id,
    phase,
    actualDuration: Number(actualDuration.toFixed(2)),
    baseDuration: Number(baseDuration.toFixed(2)),
    startTime: Number(startTime.toFixed(2)),
    commitTime: Number(commitTime.toFixed(2)),
  });
}

function App() {
  const [queryClient] = useState(() => createQueryClient());

  return (
    <ErrorBoundary name="global">
      <QueryClientProvider client={queryClient}>
        <ToastProvider>
          {import.meta.env.DEV ? (
            <Profiler id="identity-web" onRender={handleRenderProfiler}>
              <RouterProvider router={router} />
              <TrackingConsentBanner />
              <ApiConnectionOverlay />
              <VersionMismatchBanner
                appName="identity-web"
                frontendBuildId={IDENTITY_WEB_BUILD_ID}
                healthUrl={IDENTITY_HEALTH_URL}
              />
              <ReactQueryDevtools initialIsOpen={false} />
              <TanStackRouterDevtools router={router} position="bottom-right" />
            </Profiler>
          ) : (
            <>
              <RouterProvider router={router} />
              <TrackingConsentBanner />
              <ApiConnectionOverlay />
              <VersionMismatchBanner
                appName="identity-web"
                frontendBuildId={IDENTITY_WEB_BUILD_ID}
                healthUrl={IDENTITY_HEALTH_URL}
              />
            </>
          )}
        </ToastProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  );
}

export default App;
