// @vitest-environment happy-dom

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test';

const mocks = vi.hoisted(() => ({
  checkAvailability: vi.fn(),
}));

vi.mock('../src/identity.auth.queries', () => ({
  identityAuthQueryKeys: {
    registrationAvailability: (field: string, value: string) => [
      'registration-availability',
      field,
      value,
    ],
  },
  registrationAvailabilityQueryFn: mocks.checkAvailability,
}));

import { useRegisterPageAvailability } from '../src/pages/useRegisterPage.availability';

describe('useRegisterPageAvailability', () => {
  beforeEach(() => {
    mocks.checkAvailability.mockReset();
  });

  it('debounces a valid value and exposes database availability', async () => {
    mocks.checkAvailability.mockResolvedValue({ available: false });
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    const { result, rerender } = renderHook(
      ({ email, emailValid }) =>
        useRegisterPageAvailability({
          email,
          emailValid,
          username: '',
          usernameValid: false,
        }),
      {
        initialProps: { email: '', emailValid: false },
        wrapper,
      },
    );

    rerender({ email: ' User@Example.COM ', emailValid: true });

    expect(result.current.emailAvailability).toBe('checking');
    expect(mocks.checkAvailability).not.toHaveBeenCalled();

    await waitFor(() => {
      expect(result.current.emailAvailability).toBe('unavailable');
    });
    expect(mocks.checkAvailability).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({
        field: 'email',
        value: 'user@example.com',
        signal: expect.any(AbortSignal),
      }),
    );
  });
});
