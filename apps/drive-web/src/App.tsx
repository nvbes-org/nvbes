import { createQueryClient, ErrorBoundary } from '@nvbes/web-runtime';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/router-devtools';
import { Profiler, useState } from 'react';
import { NetworkQualityInit } from './components/NetworkQualityInit';
import { router } from './drive.router';

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
        <NetworkQualityInit />
        {import.meta.env.DEV ? (
          <Profiler id="drive-web" onRender={handleRenderProfiler}>
            <RouterProvider router={router} />
            <ReactQueryDevtools initialIsOpen={false} />
            <TanStackRouterDevtools router={router} position="bottom-right" />
          </Profiler>
        ) : (
          <RouterProvider router={router} />
        )}
      </QueryClientProvider>
    </ErrorBoundary>
  );
}

export default App;
