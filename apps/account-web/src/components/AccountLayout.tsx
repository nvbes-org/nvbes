import { Outlet, useNavigate } from '@tanstack/react-router';
import { useEffect, useState, useTransition } from 'react';
import { AccountSidebar } from '@/components/AccountSidebar';
import { IdentityTopBar } from '@/components/IdentityTopBar';
import { accountAuthenticationDisposition } from '@/components/account-layout.authentication';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { useAccountContext } from '@/hooks/useAccountContext';
import { captureAnalyticsException } from '@/identity.analytics';
import {
  DEFAULT_CONSENT,
  TRACKING_CONSENT_CHANGED_EVENT,
  getTrackingConsent,
  setTrackingConsent,
} from '@/tracking-consent';

function LayoutSkeleton() {
  return (
    <div className="flex h-[calc(100vh-3.5rem)]">
      <aside className="hidden w-60 shrink-0 md:flex md:flex-col">
        <div className="flex-1 px-3 flex flex-col gap-4">
          {Array.from({ length: 10 }).map((_, i) => (
            <Skeleton key={i} className="h-6 w-full rounded-lg" />
          ))}
        </div>
      </aside>
      <div className="flex flex-1 flex-col min-w-0">
        <main className="flex-1 overflow-auto">
          <div className="flex flex-col gap-10">
            <div className="flex flex-col gap-4 mx-auto w-full max-w-2xl px-4 md:px-8">
              <Skeleton className="h-13 w-120" />
              <Skeleton className="h-6 w-120" />
            </div>
            <div className="flex flex-col gap-4 mx-auto w-full max-w-2xl px-4 md:px-8">
              <Skeleton className="h-8 w-120" />
              <Skeleton className="h-8 w-120" />
              <Skeleton className="h-8 w-120" />
              <Skeleton className="h-8 w-120" />
            </div>
          </div>
        </main>
      </div>
    </div>
  );
}

export default function AccountLayout() {
  const navigate = useNavigate();
  const [isPending, startTransition] = useTransition();
  const { me, loading, error, retry, retrying } = useAccountContext();
  const authenticationDisposition = accountAuthenticationDisposition(error);
  const reauthenticationRequired = authenticationDisposition === 'reauthenticate';
  const blockingError = error && (reauthenticationRequired || !me) ? error : null;

  const posthogConfigured = Boolean(import.meta.env.VITE_POSTHOG_KEY);
  const sentryConfigured = Boolean(import.meta.env.VITE_SENTRY_DSN);
  // Default to true for Sentry if neither env var is defined in local dev
  const isSentryActive = sentryConfigured || !posthogConfigured;
  const isPosthogActive = posthogConfigured;
  const isReporterConfigured = isSentryActive || isPosthogActive;

  const [consentState, setConsentState] = useState(() => getTrackingConsent());

  useEffect(() => {
    const handleConsentChange = () => {
      setConsentState(getTrackingConsent());
    };
    window.addEventListener(TRACKING_CONSENT_CHANGED_EVENT, handleConsentChange);
    return () => window.removeEventListener(TRACKING_CONSENT_CHANGED_EVENT, handleConsentChange);
  }, []);

  useEffect(() => {
    if (!loading && authenticationDisposition === 'redirect-login') {
      void navigate({ to: '/login', replace: true });
    }
  }, [authenticationDisposition, loading, navigate]);

  const posthogAccepted = Boolean(
    consentState?.vendors?.posthog || consentState?.categories?.analytics,
  );
  const sentryAccepted = Boolean(
    consentState?.vendors?.sentry || consentState?.categories?.performance,
  );
  const isReporterAccepted =
    (isPosthogActive && posthogAccepted) || (isSentryActive && sentryAccepted);

  const enableConsent = () => {
    const baseConsent = consentState ?? DEFAULT_CONSENT;
    const nextConsent = {
      ...baseConsent,
      categories: {
        ...baseConsent.categories,
        ...(isPosthogActive ? { analytics: true } : {}),
        ...(isSentryActive ? { performance: true } : {}),
      },
      vendors: {
        ...baseConsent.vendors,
        ...(isPosthogActive ? { posthog: true } : {}),
        ...(isSentryActive ? { sentry: true } : {}),
      },
      analytics: {
        ...baseConsent.analytics,
        ...(isPosthogActive ? { productAnalytics: true } : {}),
      },
    };
    setTrackingConsent(nextConsent, 'account-web:reporter-enable');
    setConsentState(nextConsent);
  };

  return (
    <div className="min-h-screen bg-background">
      <IdentityTopBar />
      {loading ? (
        <LayoutSkeleton />
      ) : blockingError ? (
        <div className="flex h-[calc(100vh-3.5rem)] items-center justify-center">
          <div className="flex flex-col gap-3 text-center -mt-[3.5rem] w-fit mx-auto">
            <h1 className="text-3xl text-muted-foreground">
              {reauthenticationRequired
                ? 'Votre session a expiré.'
                : 'Impossible de charger votre compte.'}
            </h1>
            <div className="flex items-center gap-2 w-full">
              <Button
                variant="default"
                className="flex-1"
                onClick={() => {
                  if (!reauthenticationRequired) {
                    retry();
                    return;
                  }
                  startTransition(() => {
                    void navigate({ to: '/login' });
                  });
                }}
                disabled={isPending || retrying}
              >
                {reauthenticationRequired
                  ? isPending
                    ? 'Redirection...'
                    : 'Se reconnecter'
                  : retrying
                    ? 'Nouvelle tentative...'
                    : 'Réessayer'}
              </Button>
              {isReporterConfigured && (
                <Button
                  variant="outline"
                  className="flex-1"
                  disabled={!isReporterAccepted}
                  onClick={() => {
                    void captureAnalyticsException(
                      blockingError ?? new Error('Impossible de charger votre compte.'),
                      {
                        source: 'account_layout_retry_screen',
                        user_initiated: true,
                      },
                    );
                  }}
                >
                  Signaler
                </Button>
              )}
            </div>
            {isReporterConfigured && !isReporterAccepted && (
              <p className="text-xs text-muted-foreground">
                Le signalement nécessite d'autoriser les cookies{' '}
                <button
                  type="button"
                  onClick={enableConsent}
                  className="underline hover:text-foreground font-medium cursor-pointer"
                >
                  {isPosthogActive && isSentryActive
                    ? 'PostHog ou Sentry'
                    : isPosthogActive
                      ? 'PostHog'
                      : 'Sentry'}
                </button>
                .
              </p>
            )}
          </div>
        </div>
      ) : !me ? (
        <LayoutSkeleton />
      ) : (
        <div className="flex h-[calc(100vh-3.5rem)]">
          {/* Desktop sidebar */}
          <aside className="hidden w-60 shrink-0 md:flex md:flex-col">
            <AccountSidebar />
          </aside>

          {/* Mobile top bar + Sheet */}
          <div className="flex flex-1 flex-col min-w-0">
            <main className="flex-1 overflow-auto">
              <div className="mx-auto w-full max-w-2xl px-4 md:px-8">
                <Outlet />
              </div>
            </main>
          </div>
        </div>
      )}
    </div>
  );
}
