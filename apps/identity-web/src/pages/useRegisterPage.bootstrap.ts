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

export function useRegisterPageBootstrap() {
  const navigate = useNavigate();
  const location = useLocation();
  const [checkingAuth, setCheckingAuth] = useState(true);

  const oauthRequest = useMemo(
    () => readOAuthAuthorizeRequest(new URLSearchParams(location.searchStr)),
    [location.searchStr],
  );

  useEffect(() => {
    const check = async () => {
      try {
        await identityClient.getMe();
        await syncTrackingConsent();
        const pendingOauth =
          readOAuthAuthorizeRequest(new URLSearchParams(window.location.search)) ??
          readPendingOAuthAuthorizeRequest();

        if (pendingOauth) {
          await authorizeIdentitySession(null, pendingOauth);
          clearPendingOAuthAuthorizeRequest();
          return;
        }

        void navigate({ to: '/account' });
      } catch {
        setCheckingAuth(false);
      }
    };

    void check();
  }, [navigate]);

  return {
    checkingAuth,
    oauthRequest,
  };
}
