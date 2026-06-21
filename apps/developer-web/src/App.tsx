import { createQueryClient, ErrorBoundary, VersionMismatchBanner } from '@nvbes/web-runtime';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { useState } from 'react';
import { router } from './developer.router';

const DEVELOPER_WEB_BUILD_ID = import.meta.env.VITE_NVBES_BUILD_ID || '0.1.0';
const DEVELOPER_HEALTH_URL = `${import.meta.env.VITE_IDENTITY_API_BASE_URL || 'http://localhost:4000'}/health`;

export default function App() {
  const [queryClient] = useState(() => createQueryClient());

  return (
    <ErrorBoundary name="developer-web">
      <QueryClientProvider client={queryClient}>
        <VersionMismatchBanner
          appName="developer-web"
          frontendBuildId={DEVELOPER_WEB_BUILD_ID}
          healthUrl={DEVELOPER_HEALTH_URL}
        />
        <RouterProvider router={router} />
        {import.meta.env.DEV ? (
          <>
            <ReactQueryDevtools initialIsOpen={false} />
            <TanStackRouterDevtools router={router} position="bottom-right" />
          </>
        ) : null}
      </QueryClientProvider>
    </ErrorBoundary>
  );
}
