import { createQueryClient, ErrorBoundary } from '@nvbes/web-runtime';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { useState } from 'react';
import { router } from './developer.router';

const REACT_QUERY_DEVTOOLS_ENABLED =
  import.meta.env.DEV && import.meta.env.VITE_REACT_QUERY_DEVTOOLS_ENABLED !== 'false';
const TANSTACK_ROUTER_DEVTOOLS_ENABLED =
  import.meta.env.DEV && import.meta.env.VITE_TANSTACK_ROUTER_DEVTOOLS_ENABLED !== 'false';

export default function App() {
  const [queryClient] = useState(() => createQueryClient());

  return (
    <ErrorBoundary name="console-web">
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        {REACT_QUERY_DEVTOOLS_ENABLED ? <ReactQueryDevtools initialIsOpen={false} /> : null}
        {TANSTACK_ROUTER_DEVTOOLS_ENABLED ? (
          <TanStackRouterDevtools router={router} position="bottom-right" />
        ) : null}
      </QueryClientProvider>
    </ErrorBoundary>
  );
}
