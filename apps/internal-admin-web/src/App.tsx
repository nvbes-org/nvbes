import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState } from 'react';
import { BillingOperationsPage } from './internal-admin.billing.page';
import { InternalAdminErrorBoundary } from './internal-admin.error-boundary';

export function App() {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { retry: 1, staleTime: 15_000 },
          mutations: { retry: false },
        },
      }),
  );

  return (
    <InternalAdminErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <BillingOperationsPage />
      </QueryClientProvider>
    </InternalAdminErrorBoundary>
  );
}
