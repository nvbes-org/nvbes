import { identityClient } from '@nvbes/identity-client';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useEffect, useMemo, useState } from 'react';
import {
  authorizeIdentitySession,
  clearPendingOAuthAuthorizeRequest,
  readOAuthAuthorizeRequest,
  readPendingOAuthAuthorizeRequest,
} from '../identity.oauth';
import { syncTrackingConsent } from '../tracking-consent';

let registerSessionProbe: Promise<boolean> | null = null;

function probeRegisterSession(): Promise<boolean> {
  if (registerSessionProbe) {
    return registerSessionProbe;
  }

  registerSessionProbe = identityClient
    .getMe()
    .then(async () => {
      await syncTrackingConsent();
      return true;
    })
    .catch(() => false)
    .finally(() => {
      registerSessionProbe = null;
    });

  return registerSessionProbe;
}

export function useRegisterPageBootstrap() {
  const navigate = useNavigate();
  const location = useLocation();
  const [checkingAuth, setCheckingAuth] = useState(true);

  const oauthRequest = useMemo(
    () => readOAuthAuthorizeRequest(new URLSearchParams(location.searchStr)),
    [location.searchStr],
  );

  useEffect(() => {
    let cancelled = false;

    const check = async () => {
      const hasSession = await probeRegisterSession();
      if (cancelled) {
        return;
      }

      if (!hasSession) {
        setCheckingAuth(false);
        return;
      }

      const pendingOauth =
        readOAuthAuthorizeRequest(new URLSearchParams(window.location.search)) ??
        readPendingOAuthAuthorizeRequest();

      if (pendingOauth) {
        await authorizeIdentitySession(null, pendingOauth);
        clearPendingOAuthAuthorizeRequest();
        return;
      }

      void navigate({ to: '/account' });
    };

    void check();

    return () => {
      cancelled = true;
    };
  }, [navigate]);

  return {
    checkingAuth,
    oauthRequest,
  };
}
