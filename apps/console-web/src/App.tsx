import { createQueryClient, ErrorBoundary, VersionMismatchBanner } from '@nvbes/web-runtime';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { useState } from 'react';
import { router } from './developer.router';

const DEVELOPER_WEB_BUILD_ID = import.meta.env.VITE_NVBES_BUILD_ID || '0.1.0';
const DEVELOPER_HEALTH_URL = `${import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL || 'http://localhost:4000'}/health`;
const REACT_QUERY_DEVTOOLS_ENABLED =
  import.meta.env.DEV && import.meta.env.VITE_REACT_QUERY_DEVTOOLS_ENABLED !== 'false';
const TANSTACK_ROUTER_DEVTOOLS_ENABLED =
  import.meta.env.DEV && import.meta.env.VITE_TANSTACK_ROUTER_DEVTOOLS_ENABLED !== 'false';

export default function App() {
  const [queryClient] = useState(() => createQueryClient());

  return (
    <ErrorBoundary name="console-web">
      <QueryClientProvider client={queryClient}>
        <VersionMismatchBanner
          appName="console-web"
          frontendBuildId={DEVELOPER_WEB_BUILD_ID}
          healthUrl={DEVELOPER_HEALTH_URL}
        />
        <RouterProvider router={router} />
        {REACT_QUERY_DEVTOOLS_ENABLED ? <ReactQueryDevtools initialIsOpen={false} /> : null}
        {TANSTACK_ROUTER_DEVTOOLS_ENABLED ? (
          <TanStackRouterDevtools router={router} position="bottom-right" />
        ) : null}
      </QueryClientProvider>
    </ErrorBoundary>
  );
}
