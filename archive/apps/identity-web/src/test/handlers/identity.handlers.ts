import { http, HttpResponse } from 'msw';

export interface RegistrationAvailabilityResponse {
  available: boolean;
}

export interface RegisterSuccessResponse {
  ok: boolean;
  requiresVerification: boolean;
}

export interface ErrorResponse {
  code: string;
  message: string;
}

export const identityHandlers = [
  // Check email availability during registration
  http.get('*/auth/registration-availability', ({ request }) => {
    const url = new URL(request.url);
    const email = url.searchParams.get('email');

    if (!email || !email.includes('@')) {
      return HttpResponse.json<ErrorResponse>(
        { code: 'INVALID_EMAIL', message: 'Adresse email invalide.' },
        { status: 400 },
      );
    }

    const lower = email.toLowerCase().trim();
    if (lower.startsWith('pris@') || lower.startsWith('existant@')) {
      return HttpResponse.json<RegistrationAvailabilityResponse>({ available: false });
    }

    if (lower.startsWith('error@')) {
      return HttpResponse.json<ErrorResponse>(
        { code: 'INTERNAL_ERROR', message: 'Erreur interne du serveur.' },
        { status: 500 },
      );
    }

    return HttpResponse.json<RegistrationAvailabilityResponse>({ available: true });
  }),

  // Register a new user
  http.post('*/auth/register', async ({ request }) => {
    const payload = (await request.json()) as { email?: string; password?: string };

    if (!payload.email || !payload.password) {
      return HttpResponse.json<ErrorResponse>(
        { code: 'MISSING_FIELDS', message: 'Champs obligatoires manquants.' },
        { status: 400 },
      );
    }

    if (payload.email.startsWith('pris@')) {
      return HttpResponse.json<ErrorResponse>(
        { code: 'EMAIL_ALREADY_EXISTS', message: 'Cet email est déjà utilisé.' },
        { status: 409 },
      );
    }

    return HttpResponse.json<RegisterSuccessResponse>(
      { ok: true, requiresVerification: true },
      { status: 201 },
    );
  }),

  // Check authenticated accounts
  http.get('*/auth/accounts', () => {
    return HttpResponse.json({ accounts: [] }, { status: 200 });
  }),

  // Logout
  http.post('*/auth/logout', () => {
    return HttpResponse.json({ ok: true }, { status: 200 });
  }),
];
