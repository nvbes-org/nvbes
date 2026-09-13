import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterContextProvider,
} from '@tanstack/react-router';
import { render, type RenderOptions } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';

export function createTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
        staleTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  });
}

export function createTestRouter(initialEntries: string[] = ['/']) {
  return createRouter({
    routeTree: createRootRoute(),
    history: createMemoryHistory({ initialEntries }),
  });
}

export interface ProviderWrapperOptions {
  queryClient?: QueryClient;
  initialEntries?: string[];
}

export function createAllProviders(options: ProviderWrapperOptions = {}) {
  const queryClient = options.queryClient ?? createTestQueryClient();
  const router = createTestRouter(options.initialEntries);

  return function ProviderWrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <RouterContextProvider router={router}>{children}</RouterContextProvider>
      </QueryClientProvider>
    );
  };
}

export function renderWithProviders(
  ui: ReactElement,
  options: RenderOptions & ProviderWrapperOptions = {},
) {
  const { queryClient, initialEntries, ...renderOptions } = options;
  const client = queryClient ?? createTestQueryClient();
  const Wrapper = createAllProviders({ queryClient: client, initialEntries });

  return {
    ...render(ui, { wrapper: Wrapper, ...renderOptions }),
    queryClient: client,
  };
}
