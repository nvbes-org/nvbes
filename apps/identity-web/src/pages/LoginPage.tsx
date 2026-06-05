import { identityClient, type AccountEntry } from '@nvbes/identity-client';
import type { WebauthnRequestOptionsJSON } from '@nvbes/identity-sdk-core/src/types';
import {
  createDecoyLinks,
  type DecoyLinkTracker,
  fetchPowChallenge,
  getStoredPasswordCredential,
  getWebAuthnCredential,
  parseRequestOptions as parseLoginWebAuthnRequestOptions,
  preventAutoSignIn,
  serializeCredential,
  solvePowChallenge,
  storePasswordCredential,
  WebauthnBrowserError,
} from '@nvbes/identity-sdk-web';
import { clientErrorMessage } from '@nvbes/web-runtime';
import { useMutation } from '@tanstack/react-query';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { KeyRoundIcon, MailIcon, ShieldCheckIcon, Plus } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';

function isPasswordExpiredError(err: unknown): boolean {
  const body = (err as { body?: unknown })?.body;
  if (typeof body !== 'object' || body === null) return false;
  const error = (body as Record<string, unknown>).error;
  if (typeof error !== 'object' || error === null) return false;
  return (error as Record<string, unknown>).code === 'password_expired';
}

function isConsentRequiredError(err: unknown): boolean {
  const body = (err as { body?: unknown })?.body;
  if (typeof body !== 'object' || body === null) return false;
  const error = (body as Record<string, unknown>).error;
  if (typeof error !== 'object' || error === null) return false;
  return (error as Record<string, unknown>).code === 'consent_required';
}

const SCOPE_DESCRIPTIONS: Record<string, { label: string; desc: string }> = {
  openid: { label: 'Authentification', desc: 'Confirmer votre identité' },
  profile: { label: 'Profil utilisateur', desc: 'Accéder à votre nom et photo de profil' },
  email: { label: 'Adresse email', desc: 'Voir votre adresse email principale' },
  offline_access: {
    label: 'Accès hors connexion',
    desc: 'Maintenir la connexion sans vous reconnecter',
  },
  'drive:read': { label: 'Lecture Drive', desc: 'Consulter et lire vos fichiers et dossiers' },
  'drive:write': {
    label: 'Écriture Drive',
    desc: 'Créer, modifier et organiser vos fichiers et dossiers',
  },
  'drive:admin': {
    label: 'Administration Drive',
    desc: 'Gérer tous les paramètres de votre espace de stockage',
  },
};

import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import {
  identityAuthMutationKeys,
  startLoginWebAuthnMutationFn,
  submitLoginIdentifierMutationFn,
  submitLoginMfaMutationFn,
  submitLoginPasswordMutationFn,
} from '../identity.auth.queries';
import { identityApiBaseUrl } from '../identity.http';
import {
  authorizeIdentitySession,
  clearPendingOAuthAuthorizeRequest,
  readOAuthAuthorizeRequest,
  readPendingOAuthAuthorizeRequest,
} from '../identity.oauth';
import { LoginBrandPanel } from './LoginBrandPanel';
import { LoginProgress, type LoginStep } from './LoginProgress';

type MfaMethod = 'totp' | 'webauthn' | 'recovery';

function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

export default function LoginPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const oauthRequest = useMemo(() => {
    const searchParams = new URLSearchParams(location.searchStr);
    return readOAuthAuthorizeRequest(searchParams) ?? readPendingOAuthAuthorizeRequest();
  }, [location.searchStr]);

  const [checkingAuth, setCheckingAuth] = useState(true);
  const [connectedAccounts, setConnectedAccounts] = useState<AccountEntry[]>([]);

  useEffect(() => {
    const check = async () => {
      try {
        const accountsList = await identityClient.listAccounts().catch(() => []);
        setConnectedAccounts(accountsList);

        const searchParams = new URLSearchParams(location.searchStr);
        const hasAuthUserQuery = searchParams.has('authuser');

        if (hasAuthUserQuery) {
          await identityClient.getMe();
          if (oauthRequest) {
            await authorizeIdentitySession(null, oauthRequest);
            clearPendingOAuthAuthorizeRequest();
            return;
          }
          void navigate({ to: '/account' });
          return;
        }

        if (accountsList.length > 0) {
          setStep('chooser');
          setCheckingAuth(false);
          return;
        }

        await identityClient.getMe();
        if (oauthRequest) {
          await authorizeIdentitySession(null, oauthRequest);
          clearPendingOAuthAuthorizeRequest();
          return;
        }
        void navigate({ to: '/account' });
      } catch (err) {
        if (isConsentRequiredError(err)) {
          setStep('consent');
        } else {
          setError(clientErrorMessage(err, "Impossible de finaliser l'autorisation OAuth"));
        }
        setCheckingAuth(false);
      }
    };
    void check();
  }, [navigate, oauthRequest]);

  const handleAccountSelect = async (authuser: string) => {
    const url = new URL(window.location.href);
    url.searchParams.set('authuser', authuser);
    window.location.href = url.toString();
  };

  const handleUseAnotherAccount = async () => {
    setError(null);
    let nextAuthUser = 0;
    const activeSlots = connectedAccounts
      .map((a) => parseInt(a.authuser, 10))
      .filter((n) => !Number.isNaN(n));
    if (activeSlots.length > 0) {
      nextAuthUser = Math.max(...activeSlots) + 1;
    }

    await (
      navigate as (opts: {
        search: (prev: Record<string, unknown>) => Record<string, unknown>;
      }) => Promise<void>
    )({
      search: (prev: Record<string, unknown>) => ({
        ...prev,
        authuser: String(nextAuthUser),
      }),
    });
    const url = new URL(window.location.href);
    url.searchParams.set('authuser', String(nextAuthUser));
    window.history.replaceState(null, '', url.toString());

    setEmail('');
    setPassword('');
    setStep('identifier');
  };

  useEffect(() => {
    if (checkingAuth) return;
    const tryAutoFill = async () => {
      const stored = await getStoredPasswordCredential();
      if (stored) {
        setEmail(stored.id);
        setPassword(stored.password);
      }
    };
    void tryAutoFill();
  }, [checkingAuth]);

  const decoyRef = useRef<DecoyLinkTracker | null>(null);

  useEffect(() => {
    const container = document.getElementById('decoy-links-container');
    if (!container) return;
    decoyRef.current = createDecoyLinks(container, 3);
    return () => decoyRef.current?.destroy();
  }, []);

  const [step, setStep] = useState<LoginStep>('identifier');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [loginStateToken, setLoginStateToken] = useState<string | null>(null);
  const [sessionToken, setSessionToken] = useState<string | null>(null);
  const [availableMethods, setAvailableMethods] = useState<MfaMethod[]>([]);
  const [mfaMethod, setMfaMethod] = useState<MfaMethod | null>(null);
  const [totpCode, setTotpCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');
  const [error, setError] = useState<string | null>(null);

  const loginIdentifierMutation = useMutation({
    mutationKey: identityAuthMutationKeys.loginIdentifier,
    mutationFn: submitLoginIdentifierMutationFn,
  });
  const loginPasswordMutation = useMutation({
    mutationKey: identityAuthMutationKeys.loginPassword,
    mutationFn: submitLoginPasswordMutationFn,
  });
  const loginMfaMutation = useMutation({
    mutationKey: identityAuthMutationKeys.loginMfa,
    mutationFn: submitLoginMfaMutationFn,
  });
  const loginWebauthnStartMutation = useMutation({
    mutationKey: identityAuthMutationKeys.loginWebauthnStart,
    mutationFn: startLoginWebAuthnMutationFn,
  });

  const webauthnTimeoutMs = 60_000;
  const loading =
    loginIdentifierMutation.isPending ||
    loginPasswordMutation.isPending ||
    loginMfaMutation.isPending ||
    loginWebauthnStartMutation.isPending;

  const resetMfaState = () => {
    setAvailableMethods([]);
    setMfaMethod(null);
    setTotpCode('');
    setRecoveryCode('');
  };

  const finishLogin = async (session: string | null) => {
    storePasswordCredential(email, password, email).catch(() => {});

    if (oauthRequest) {
      if (!session) {
        throw new Error('Login did not return a session token.');
      }
      try {
        await authorizeIdentitySession(session, oauthRequest);
        clearPendingOAuthAuthorizeRequest();
      } catch (err) {
        if (isConsentRequiredError(err)) {
          setSessionToken(session);
          setStep('consent');
        } else {
          throw err;
        }
      }
      return;
    }
    void navigate({ to: '/account' });
  };

  const handleConsentApprove = async () => {
    if (!oauthRequest) return;
    setError(null);
    setCheckingAuth(true);
    try {
      await authorizeIdentitySession(sessionToken, {
        ...oauthRequest,
        consentAction: 'approve',
      });
      clearPendingOAuthAuthorizeRequest();
    } catch (err) {
      setError(clientErrorMessage(err, 'Failed to grant consent'));
      setCheckingAuth(false);
    }
  };

  const handleConsentCancel = () => {
    if (oauthRequest) {
      const url = new URL(oauthRequest.redirectUri);
      url.searchParams.set('error', 'access_denied');
      url.searchParams.set('error_description', 'The user denied consent.');
      if (oauthRequest.state) {
        url.searchParams.set('state', oauthRequest.state);
      }
      clearPendingOAuthAuthorizeRequest();
      window.location.assign(url.toString());
    } else {
      void navigate({ to: '/account' });
    }
  };

  const handleIdentifierSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError(null);
    try {
      const challenge = await fetchPowChallenge(identityApiBaseUrl);
      const powResolved =
        challenge && challenge.difficulty > 0
          ? {
              powNonce: challenge.nonce,
              powSolution: String(await solvePowChallenge(challenge.nonce, challenge.difficulty)),
            }
          : {};

      const id = await loginIdentifierMutation.mutateAsync({
        email,
        decoy_link_clicked: decoyRef.current?.wasClicked() ?? false,
        ...powResolved,
      });
      setLoginStateToken(id.state_token);
      setSessionToken(null);
      resetMfaState();

      const requestedMfa = id.next_step === 'mfa' || id.available_methods?.length;
      if (requestedMfa) {
        setAvailableMethods(
          (id.available_methods ?? [])
            .map((m) => m as MfaMethod)
            .filter((m): m is MfaMethod => m === 'totp' || m === 'webauthn' || m === 'recovery'),
        );
        if (id.available_methods?.includes('webauthn')) {
          setMfaMethod('webauthn');
        } else {
          setMfaMethod(null);
        }
        setStep('mfa');
      } else {
        setStep('password');
      }
    } catch (err) {
      setError(clientErrorMessage(err, 'Login failed'));
    }
  };

  const handlePasswordSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!loginStateToken) {
      setError('Session de connexion expirée. Recommencez.');
      setStep('identifier');
      return;
    }
    setError(null);
    try {
      const data = await loginPasswordMutation.mutateAsync({
        stateToken: loginStateToken,
        password,
      });
      const requestedMfa = data.next_step === 'mfa' || data.available_methods?.length;
      if (requestedMfa) {
        if (data.state_token) {
          setLoginStateToken(data.state_token);
        }
        setAvailableMethods(
          (data.available_methods ?? [])
            .map((m) => m as MfaMethod)
            .filter((m): m is MfaMethod => m === 'totp' || m === 'webauthn' || m === 'recovery'),
        );
        setMfaMethod(null);
        setStep('mfa');
        return;
      }
      if (data.user?.email_verified === false) {
        void navigate({
          to: '/verify',
          state: (s) => ({
            ...s,
            email: data.user?.email ?? email,
            resendAvailableAt: data.verification_resend_available_at ?? null,
          }),
        });
        return;
      }
      if (data.session_token) setSessionToken(data.session_token);
      await finishLogin(data.session_token ?? null);
    } catch (err) {
      if (isPasswordExpiredError(err)) {
        void navigate({
          to: '/forgot-password',
          state: (s) => ({ ...s, email }),
        });
        return;
      }
      setError(clientErrorMessage(err, 'Login failed'));
    }
  };

  const handleMfaSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!loginStateToken) {
      setError('Session de connexion expirée. Recommencez.');
      setStep('identifier');
      return;
    }
    if (!mfaMethod) return;
    setError(null);
    try {
      let result: Awaited<ReturnType<typeof loginMfaMutation.mutateAsync>> | undefined;
      if (mfaMethod === 'totp') {
        result = await loginMfaMutation.mutateAsync({ stateToken: loginStateToken, totpCode });
      } else if (mfaMethod === 'recovery') {
        result = await loginMfaMutation.mutateAsync({ stateToken: loginStateToken, recoveryCode });
      } else {
        const start = await loginWebauthnStartMutation.mutateAsync(loginStateToken);
        const options = parseLoginWebAuthnRequestOptions(
          start.options as WebauthnRequestOptionsJSON | { publicKey: WebauthnRequestOptionsJSON },
        );
        const credential = await getWebAuthnCredential(options, { timeoutMs: webauthnTimeoutMs });
        result = await loginMfaMutation.mutateAsync({
          stateToken: loginStateToken,
          webauthnChallengeId: start.challenge_id,
          webauthnResponse: serializeCredential(credential),
        });
      }
      if (result?.session_token) setSessionToken(result.session_token);
      await finishLogin(result?.session_token ?? sessionToken);
    } catch (err) {
      if (err instanceof WebauthnBrowserError && err.code === 'webauthn_timeout') {
        setError('WebAuthn request timed out. Please try again.');
      } else {
        setError(err instanceof Error ? err.message : 'MFA verification failed');
      }
    }
  };

  const hasTotp = availableMethods.includes('totp');
  const hasWebAuthn = availableMethods.includes('webauthn');
  const hasRecovery = availableMethods.includes('recovery');
  const resetToIdentifier = () => {
    preventAutoSignIn().catch(() => {});
    setStep('identifier');
    setLoginStateToken(null);
    setSessionToken(null);
    resetMfaState();
    setError(null);
    setPassword('');
  };

  return (
    <div className="flex min-h-screen">
      {checkingAuth ? (
        <div className="flex flex-1 items-center justify-center bg-muted/30">
          <div className="size-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        </div>
      ) : (
        <>
          <LoginBrandPanel />

          {/* Form Panel */}
          <div className="flex flex-1 items-center justify-center bg-muted/30 p-4 sm:p-8">
            <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
              {/* Mobile-only brand header */}
              <div className="mb-8 flex items-center gap-2.5 lg:hidden">
                <div className="inline-flex size-8 items-center justify-center rounded-lg bg-primary/15">
                  <div className="size-3 rounded-sm bg-primary" />
                </div>
                <span className="text-lg font-semibold tracking-tight text-foreground/85">
                  nvbes
                </span>
              </div>

              <LoginProgress step={step} />

              {/* Main Card */}
              <Card>
                <CardHeader>
                  <CardTitle>
                    {step === 'consent'
                      ? "Demande d'autorisation"
                      : step === 'chooser'
                        ? 'Choisir un compte'
                        : 'Connexion'}
                  </CardTitle>
                  <CardDescription>
                    {step === 'identifier'
                      ? 'Entrez votre email pour commencer.'
                      : step === 'password'
                        ? 'Saisissez votre mot de passe.'
                        : step === 'consent'
                          ? `L'application souhaite accéder à votre compte.`
                          : step === 'chooser'
                            ? 'pour continuer sur nvbes'
                            : 'Vérification en deux étapes.'}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {step === 'chooser' && (
                    <div className="flex flex-col gap-4">
                      <div className="flex flex-col gap-2 max-h-[280px] overflow-y-auto pr-1">
                        {connectedAccounts.map((account) => {
                          const initials = (account.user.display_name || account.user.email || '?')
                            .slice(0, 1)
                            .toUpperCase();
                          return (
                            <button
                              key={account.authuser}
                              type="button"
                              onClick={() => handleAccountSelect(account.authuser)}
                              className="flex w-full items-center gap-3 rounded-xl border border-border/60 bg-card p-3 text-left transition hover:bg-muted hover:border-primary/20"
                            >
                              <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary text-sm font-semibold">
                                {initials}
                              </div>
                              <div className="flex flex-col min-w-0">
                                <span className="text-sm font-medium truncate">
                                  {account.user.display_name}
                                </span>
                                <span className="text-xs text-muted-foreground truncate">
                                  {account.user.email}
                                </span>
                              </div>
                            </button>
                          );
                        })}
                      </div>

                      <Separator />

                      <Button
                        type="button"
                        variant="outline"
                        onClick={handleUseAnotherAccount}
                        className="w-full flex items-center justify-center gap-2"
                      >
                        <Plus className="size-4" />
                        Se connecter à un autre compte
                      </Button>
                    </div>
                  )}

                  {step === 'identifier' && (
                    <form onSubmit={handleIdentifierSubmit} className="flex flex-col gap-5">
                      <div className="flex flex-col gap-2">
                        <Label htmlFor="login-email">Email</Label>
                        <Input
                          id="login-email"
                          type="email"
                          placeholder="vous@exemple.fr"
                          value={email}
                          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                            setEmail(e.target.value)
                          }
                          required
                          autoComplete="email"
                          autoFocus
                        />
                      </div>
                      {error && <ErrorMessage message={error} />}
                      <Button type="submit" disabled={loading} className="w-full" size="lg">
                        {loading ? 'Recherche...' : 'Continuer'}
                      </Button>
                    </form>
                  )}

                  {step === 'password' && (
                    <form onSubmit={handlePasswordSubmit} className="flex flex-col gap-5">
                      <div className="flex items-center gap-2 rounded-lg border bg-muted/50 px-3 py-2">
                        <MailIcon className="size-4 shrink-0 text-muted-foreground" />
                        <span className="text-sm font-medium truncate">{email}</span>
                      </div>
                      <div className="flex flex-col gap-2">
                        <Label htmlFor="login-password">Mot de passe</Label>
                        <Input
                          id="login-password"
                          type="password"
                          placeholder="••••••••"
                          value={password}
                          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                            setPassword(e.target.value)
                          }
                          required
                          autoComplete="current-password"
                          autoFocus
                        />
                      </div>
                      <div className="flex justify-end">
                        <a
                          href="/forgot-password"
                          className="text-xs text-muted-foreground hover:text-foreground transition-colors"
                        >
                          Mot de passe oublie ?
                        </a>
                      </div>
                      {error && <ErrorMessage message={error} />}
                      <div className="flex gap-2">
                        <Button
                          type="button"
                          variant="outline"
                          className="flex-1"
                          onClick={resetToIdentifier}
                          disabled={loading}
                        >
                          Retour
                        </Button>
                        <Button type="submit" disabled={loading} className="flex-1">
                          {loading ? 'Connexion...' : 'Se connecter'}
                        </Button>
                      </div>
                    </form>
                  )}

                  {step === 'mfa' && (
                    <MfaStep
                      error={error}
                      loading={loading}
                      mfaMethod={mfaMethod}
                      hasTotp={hasTotp}
                      hasWebAuthn={hasWebAuthn}
                      hasRecovery={hasRecovery}
                      availableCount={availableMethods.length}
                      totpCode={totpCode}
                      recoveryCode={recoveryCode}
                      onTotpCodeChange={setTotpCode}
                      onRecoveryCodeChange={setRecoveryCode}
                      onMfaMethodSelect={setMfaMethod}
                      onMfaSubmit={handleMfaSubmit}
                      onBackToMethodSelect={() => {
                        setMfaMethod(null);
                        setError(null);
                      }}
                      onResetToIdentifier={resetToIdentifier}
                    />
                  )}

                  {step === 'consent' && (
                    <div className="flex flex-col gap-5">
                      <div className="rounded-2xl border bg-muted/20 p-4">
                        <p className="text-xs font-semibold text-foreground/80 mb-3 uppercase tracking-wider">
                          Autorisations demandées :
                        </p>
                        <div className="flex flex-col gap-3.5">
                          {(oauthRequest?.scope?.split(/\s+/) || []).map((sc) => {
                            const desc = SCOPE_DESCRIPTIONS[sc] || {
                              label: sc,
                              desc: "Scope requis par l'application",
                            };
                            return (
                              <div key={sc} className="flex items-start gap-2.5">
                                <div className="mt-0.5 rounded-full bg-primary/10 p-1 text-primary">
                                  <ShieldCheckIcon className="size-3.5" />
                                </div>
                                <div>
                                  <p className="text-xs font-semibold text-foreground/85">
                                    {desc.label}
                                  </p>
                                  <p className="text-[11px] text-muted-foreground">{desc.desc}</p>
                                </div>
                              </div>
                            );
                          })}
                        </div>
                      </div>
                      {error && <ErrorMessage message={error} />}
                      <div className="flex gap-2">
                        <Button
                          type="button"
                          variant="outline"
                          className="flex-1"
                          onClick={handleConsentCancel}
                        >
                          Refuser
                        </Button>
                        <Button type="button" className="flex-1" onClick={handleConsentApprove}>
                          Autoriser
                        </Button>
                      </div>
                    </div>
                  )}
                </CardContent>
                <CardFooter className="justify-center">
                  {step !== 'consent' && step !== 'chooser' && (
                    <p className="text-center text-sm text-muted-foreground">
                      Pas de compte ?{' '}
                      <a
                        href={`/register${location.searchStr}`}
                        className="font-medium text-primary hover:underline"
                      >
                        S&apos;inscrire
                      </a>
                    </p>
                  )}
                </CardFooter>
              </Card>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function MfaStep({
  error,
  loading,
  mfaMethod,
  hasTotp,
  hasWebAuthn,
  hasRecovery,
  availableCount,
  totpCode,
  recoveryCode,
  onTotpCodeChange,
  onRecoveryCodeChange,
  onMfaMethodSelect,
  onMfaSubmit,
  onBackToMethodSelect,
  onResetToIdentifier,
}: {
  error: string | null;
  loading: boolean;
  mfaMethod: MfaMethod | null;
  hasTotp: boolean;
  hasWebAuthn: boolean;
  hasRecovery: boolean;
  availableCount: number;
  totpCode: string;
  recoveryCode: string;
  onTotpCodeChange: (v: string) => void;
  onRecoveryCodeChange: (v: string) => void;
  onMfaMethodSelect: (m: MfaMethod) => void;
  onMfaSubmit: (e: React.FormEvent<HTMLFormElement>) => void;
  onBackToMethodSelect: () => void;
  onResetToIdentifier: () => void;
}) {
  return (
    <div className="flex flex-col gap-5">
      {!mfaMethod ? (
        <div className="flex flex-col gap-2">
          {hasTotp && (
            <Button
              variant="outline"
              className="w-full justify-start gap-3"
              onClick={() => onMfaMethodSelect('totp')}
            >
              <ShieldCheckIcon className="size-4 text-muted-foreground" />
              Code d&apos;authentification (TOTP)
            </Button>
          )}
          {hasWebAuthn && (
            <Button
              variant="outline"
              className="w-full justify-start gap-3"
              onClick={() => onMfaMethodSelect('webauthn')}
            >
              <KeyRoundIcon className="size-4 text-muted-foreground" />
              Clé de sécurité (WebAuthn)
            </Button>
          )}
          {hasRecovery && (
            <Button
              variant="outline"
              className="w-full justify-start gap-3"
              onClick={() => onMfaMethodSelect('recovery')}
            >
              <MailIcon className="size-4 text-muted-foreground" />
              Code de récupération
            </Button>
          )}
          {availableCount === 0 && (
            <p className="py-4 text-center text-sm text-muted-foreground">
              Aucun facteur MFA n&apos;a été proposé pour cette session.
            </p>
          )}
        </div>
      ) : (
        <form onSubmit={onMfaSubmit} className="flex flex-col gap-5">
          {mfaMethod === 'totp' && (
            <div className="flex flex-col gap-2">
              <Label htmlFor="totp-code">Code d&apos;authentification</Label>
              <Input
                id="totp-code"
                type="text"
                inputMode="numeric"
                autoComplete="one-time-code"
                placeholder="000000"
                maxLength={6}
                value={totpCode}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  onTotpCodeChange(e.target.value)
                }
                required
                autoFocus
              />
            </div>
          )}

          {mfaMethod === 'recovery' && (
            <div className="flex flex-col gap-2">
              <Label htmlFor="recovery-code">Code de récupération</Label>
              <Input
                id="recovery-code"
                type="text"
                placeholder="XXXX-XXXX-XXXX"
                value={recoveryCode}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  onRecoveryCodeChange(e.target.value)
                }
                required
                autoFocus
              />
            </div>
          )}

          {mfaMethod === 'webauthn' && (
            <p className="py-4 text-center text-sm text-muted-foreground">
              Cliquez sur Valider pour utiliser votre clé de sécurité.
            </p>
          )}

          {error && <ErrorMessage message={error} />}

          <div className="flex gap-2">
            <Button
              type="button"
              variant="outline"
              className="flex-1"
              onClick={onBackToMethodSelect}
              disabled={loading}
            >
              Retour
            </Button>
            <Button type="submit" className="flex-1" disabled={loading}>
              {loading ? 'Vérification...' : 'Valider'}
            </Button>
          </div>
        </form>
      )}

      <Separator />

      <button
        type="button"
        className="text-center text-sm text-muted-foreground hover:text-foreground transition-colors"
        onClick={onResetToIdentifier}
      >
        Pas votre compte ? Revenir à l&apos;identification
      </button>
    </div>
  );
}
