import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState } from 'react';
import { BillingOperationsPage } from './backoffice-service.billing.page';
import { BackofficeServiceErrorBoundary } from './backoffice-service.error-boundary';

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
    <BackofficeServiceErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <BillingOperationsPage />
      </QueryClientProvider>
    </BackofficeServiceErrorBoundary>
  );
}
