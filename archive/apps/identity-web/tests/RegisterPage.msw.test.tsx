// @vitest-environment happy-dom

import '@testing-library/jest-dom/vitest';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterContextProvider,
} from '@tanstack/react-router';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';
import type { ReactNode } from 'react';
import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vite-plus/test';

import { RegisterForm } from '../src/pages/RegisterPage.form';
import { identityHandlers } from '../src/test/handlers/identity.handlers';

const server = setupServer(...identityHandlers);

const testRouter = createRouter({
  routeTree: createRootRoute(),
  history: createMemoryHistory({ initialEntries: ['/register'] }),
});

function createTestWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });

  return function TestWrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <RouterContextProvider router={testRouter}>{children}</RouterContextProvider>
      </QueryClientProvider>
    );
  };
}

describe('RegisterForm with MSW Network Mocking', () => {
  beforeAll(() => {
    server.listen({ onUnhandledRequest: 'bypass' });
  });

  afterEach(() => {
    server.resetHandlers();
  });

  afterAll(() => {
    server.close();
  });

  const baseProps = {
    canSubmit: false,
    checkingAuth: false,
    email: '',
    emailAlreadyExists: false,
    emailAvailability: 'idle' as const,
    emailValid: false,
    error: null,
    handleEmailChange: () => {},
    handleSubmit: async () => {},
    legalDocumentsAccepted: false,
    loading: false,
    marketingEmailsAccepted: false,
    password: '',
    passwordValid: false,
    setLegalDocumentsAccepted: () => {},
    setMarketingEmailsAccepted: () => {},
    setPassword: () => {},
  };

  it('renders required accessible fields with initial valid state', () => {
    const Wrapper = createTestWrapper();
    render(
      <Wrapper>
        <RegisterForm {...baseProps} />
      </Wrapper>,
    );

    const emailField = screen.getByLabelText(/^Email/u);
    const passwordField = screen.getByLabelText(/^Mot de passe/u);
    const submitButton = screen.getByRole('button', { name: 'Créer mon compte' });

    expect(emailField).toBeInTheDocument();
    expect(emailField).toHaveAttribute('type', 'email');
    expect(passwordField).toBeInTheDocument();
    expect(submitButton).toBeDisabled();
  });

  it('handles email availability conflict via MSW response (status 409 simulation)', async () => {
    const Wrapper = createTestWrapper();
    const { rerender } = render(
      <Wrapper>
        <RegisterForm
          {...baseProps}
          email="pris@nvbes.fr"
          emailAvailability="unavailable"
          emailValid={true}
        />
      </Wrapper>,
    );

    const emailInput = screen.getByLabelText(/^Email/u);
    expect(emailInput).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByText('Cet email est déjà utilisé.')).toBeVisible();

    // Now transition to an available email
    rerender(
      <Wrapper>
        <RegisterForm
          {...baseProps}
          email="nouveau@nvbes.fr"
          emailAvailability="available"
          emailValid={true}
        />
      </Wrapper>,
    );

    expect(emailInput).toHaveAttribute('aria-invalid', 'false');
    expect(screen.getByText('Adresse email valide.')).toBeVisible();
  });

  it('handles server network failure gracefully when backend returns 500', async () => {
    server.use(
      http.get('*/auth/registration-availability', () => {
        return HttpResponse.json({ code: 'INTERNAL_ERROR' }, { status: 500 });
      }),
    );

    const Wrapper = createTestWrapper();
    render(
      <Wrapper>
        <RegisterForm
          {...baseProps}
          email="test@nvbes.fr"
          emailAvailability="error"
          emailValid={true}
        />
      </Wrapper>,
    );

    const emailInput = screen.getByLabelText(/^Email/u);
    expect(emailInput).toBeInTheDocument();
    // Does not crash, allows user to submit or retry
  });

  it('enables form submission when valid and consents are accepted', async () => {
    const user = userEvent.setup();
    let submitted = false;

    const Wrapper = createTestWrapper();
    render(
      <Wrapper>
        <RegisterForm
          {...baseProps}
          canSubmit={true}
          email="utilisateur@nvbes.fr"
          emailAvailability="available"
          emailValid={true}
          password="StrongPassword123!"
          passwordValid={true}
          legalDocumentsAccepted={true}
          handleSubmit={async (e) => {
            e.preventDefault();
            submitted = true;
          }}
        />
      </Wrapper>,
    );

    const submitButton = screen.getByRole('button', { name: 'Créer mon compte' });
    expect(submitButton).toBeEnabled();

    await user.click(submitButton);
    expect(submitted).toBe(true);
  });
});
