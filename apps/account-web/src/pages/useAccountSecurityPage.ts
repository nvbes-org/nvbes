import { identityClient } from '@nvbes/identity-client';
import { listMfaFactors } from '@nvbes/identity-sdk-web';
import { useMutation, useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useTransition } from 'react';
import { z } from 'zod';

import { accountQueryKeys } from '@/account.queries';
import { authuserSearch, readAuthuser } from '@/identity.authuser';
import { identityHttpClient } from '../identity.http';

const SecurityPreferencesSchema = z.object({
  theme: z.string(),
  language: z.string(),
  skip_password: z.boolean().default(false),
});

export interface SecurityOverview {
  mfa_enabled: boolean;
  email_verified: boolean;
  session_count: number;
  has_webauthn: boolean;
  skip_password: boolean;
  raw_prefs: {
    theme: string;
    language: string;
    skip_password: boolean;
  } | null;
}

export function useAccountSecurityPage() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr);
  const navigate = useNavigate();
  const [isPending, startTransition] = useTransition();
  const queryClient = useQueryClient();
  const securityOverviewQueryKey = accountQueryKeys.securityOverview(authuser);

  const { data: overview } = useSuspenseQuery({
    queryKey: securityOverviewQueryKey,
    queryFn: async ({ signal }) => {
      const [meResult, sessionsResult, factorsResult, prefsResult] = await Promise.allSettled([
        identityClient.getMe({ signal }),
        identityClient.listSessions({ signal }),
        listMfaFactors(''),
        identityHttpClient.request('/auth/me/preferences', SecurityPreferencesSchema, { signal }),
      ]);

      const me = meResult.status === 'fulfilled' ? meResult.value : null;
      const sessions = sessionsResult.status === 'fulfilled' ? sessionsResult.value : [];
      const factors = factorsResult.status === 'fulfilled' ? factorsResult.value.factors : [];
      const prefs = prefsResult.status === 'fulfilled' ? prefsResult.value : null;

      return {
        mfa_enabled: me?.user.mfa_enabled ?? false,
        email_verified: me?.user.email_verified ?? false,
        session_count: sessions.length,
        has_webauthn: factors.some((factor) => factor.factor_type === 'webauthn'),
        skip_password: prefs?.skip_password ?? false,
        raw_prefs: prefs
          ? {
              theme: prefs.theme,
              language: prefs.language,
              skip_password: prefs.skip_password ?? false,
            }
          : null,
      } satisfies SecurityOverview;
    },
    staleTime: 0,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const mutation = useMutation({
    mutationFn: async (newSkipPassword: boolean) => {
      if (!overview.raw_prefs) {
        return;
      }

      await identityHttpClient.request('/auth/me/preferences', SecurityPreferencesSchema, {
        method: 'PUT',
        body: {
          ...overview.raw_prefs,
          skip_password: newSkipPassword,
        },
      });
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: securityOverviewQueryKey });
    },
  });

  return {
    isPending,
    mutation,
    onOpenMfa: () =>
      startTransition(
        () => void navigate({ to: '/account/mfa', search: authuserSearch(authuser) }),
      ),
    onOpenPassword: () =>
      startTransition(
        () =>
          void navigate({
            to: '/account/security/password',
            search: authuserSearch(authuser),
          }),
      ),
    onOpenSessions: () =>
      startTransition(
        () => void navigate({ to: '/account/sessions', search: authuserSearch(authuser) }),
      ),
    overview,
  };
}
