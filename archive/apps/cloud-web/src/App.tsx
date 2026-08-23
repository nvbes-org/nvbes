import { createQueryClient, ErrorBoundary } from '@nvbes/web-runtime';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { Profiler, useState } from 'react';
import { NetworkQualityInit } from './components/NetworkQualityInit';
import { TooltipProvider } from './components/ui/tooltip';
import { router } from './drive.router';
import { TrackingConsentBanner } from './TrackingConsentBanner';

const REACT_QUERY_DEVTOOLS_ENABLED =
  import.meta.env.DEV && import.meta.env.VITE_REACT_QUERY_DEVTOOLS_ENABLED !== 'false';
const TANSTACK_ROUTER_DEVTOOLS_ENABLED =
  import.meta.env.DEV && import.meta.env.VITE_TANSTACK_ROUTER_DEVTOOLS_ENABLED !== 'false';

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

export function App() {
  const [queryClient] = useState(() => createQueryClient());

  return (
    <ErrorBoundary name="global">
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <NetworkQualityInit />
          <TrackingConsentBanner />
          {import.meta.env.DEV ? (
            <Profiler id="cloud-web" onRender={handleRenderProfiler}>
              <RouterProvider router={router} />
              {REACT_QUERY_DEVTOOLS_ENABLED ? <ReactQueryDevtools initialIsOpen={false} /> : null}
              {TANSTACK_ROUTER_DEVTOOLS_ENABLED ? (
                <TanStackRouterDevtools router={router} position="bottom-right" />
              ) : null}
            </Profiler>
          ) : (
            <RouterProvider router={router} />
          )}
        </TooltipProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  );
}

export default App;
