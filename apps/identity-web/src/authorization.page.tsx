import { useEffect, useState, useRef, useSyncExternalStore } from 'react';
import { ArrowUpRight } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { AuthShell } from './auth.shell';
import { LoginProgress } from './components/LoginProgress';
import { AuthenticationForms } from './authorization.forms';
import { EnrollmentForm } from './authorization.enrollment';
import { RecoveryCodes } from './authorization.recovery-codes';
import { FactorsPanel } from './factors.panel';
import { managementExpiry } from './authorization.state';
import type { AuthorizationController } from './authorization.controller';

const titles = {
  loading: 'Préparation de votre connexion',
  login: 'Votre espace commence ici.',
  reauthenticate: 'Reconnectez-vous pour continuer.',
  'step-up': 'Confirmez que c’est vous.',
  'security-step-up': 'Confirmez votre accès à la sécurité.',
  enrollment: 'Protégez votre compte.',
  'recovery-codes': 'Gardez un accès de secours.',
  factors: 'Vos méthodes de sécurité.',
  consent: 'Vous gardez le contrôle.',
  leaving: 'Retour à votre application',
  closed: 'Connexion interrompue',
};

const scopeLabels: Record<string, string> = {
  openid: 'Vous identifier',
  profile: 'Consulter votre profil',
  email: 'Consulter votre adresse email',
  'account:read': 'Consulter votre compte',
  'account:write': 'Modifier votre compte',
  'account:export': 'Exporter vos données',
  'account:close': 'Demander la fermeture de votre compte',
  'billing:read': 'Consulter votre facturation',
  'billing:write': 'Gérer votre facturation',
};

export function AuthorizationPage({ controller }: { controller: AuthorizationController }) {
  const [loginStep, setLoginStep] = useState<'identifier' | 'password'>('identifier');
  const state = useSyncExternalStore(controller.subscribe, controller.snapshot);
  const securityExpiry = managementExpiry(state);
  useEffect(() => {
    if (state.stage !== 'consent' || !securityExpiry) return;
    const expire = () => controller.expireSecurityAccess();
    const timer = window.setTimeout(expire, Math.max(0, Date.parse(securityExpiry) - Date.now()));
    document.addEventListener('visibilitychange', expire);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('visibilitychange', expire);
    };
  }, [controller, state.stage, securityExpiry]);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, [state.stage]);
  const active =
    state.interaction !== null && !['closed', 'leaving', 'loading'].includes(state.stage);
  return (
    <AuthShell
      title={
        state.stage === 'login'
          ? loginStep === 'password'
            ? 'Bienvenue'
            : 'Se connecter'
          : state.stage === 'consent'
            ? "Demande d'autorisation"
            : titles[state.stage]
      }
      headingRef={heading}
      description={
        state.stage === 'login'
          ? loginStep === 'password'
            ? 'Confirmez votre identité pour continuer vers vos services nvbes.'
            : 'Utilisez votre compte nvbes pour accéder à votre espace sécurisé.'
          : state.stage === 'consent'
            ? "Vérifiez les accès demandés par l'application."
            : undefined
      }
    >
      <div
        className="flex flex-col gap-6 animate-login-card-enter-forward motion-reduce:animate-none"
        aria-busy={state.busy}
      >
        {['login', 'reauthenticate', 'step-up', 'security-step-up'].includes(state.stage) && (
          <LoginProgress
            isOAuthFlow
            step={state.stage === 'login' || state.stage === 'reauthenticate' ? loginStep : 'mfa'}
          />
        )}
        {state.error && (
          <Alert variant="destructive">
            <AlertDescription>{state.error}</AlertDescription>
          </Alert>
        )}
        {['login', 'reauthenticate', 'step-up', 'security-step-up'].includes(state.stage) && (
          <AuthenticationForms controller={controller} state={state} onStep={setLoginStep} />
        )}
        {state.stage === 'enrollment' && <EnrollmentForm controller={controller} state={state} />}
        {state.stage === 'consent' && state.interaction && (
          <>
            {securityExpiry && (
              <>
                <p className="text-sm text-muted-foreground">
                  Générer des codes de secours remplace immédiatement tous vos anciens codes.
                </p>
                <Button
                  variant="outline"
                  disabled={state.busy}
                  onClick={() => void controller.generateRecoveryCodes()}
                >
                  Générer des codes de secours
                </Button>
              </>
            )}
          </>
        )}
        {state.stage === 'recovery-codes' && (
          <RecoveryCodes controller={controller} state={state} />
        )}
        {state.stage === 'factors' && state.interaction?.sessionCsrfToken && securityExpiry && (
          <FactorsPanel
            authorization={controller}
            csrf={state.interaction.sessionCsrfToken}
            expiresAt={securityExpiry}
          />
        )}
        {state.stage === 'consent' && state.interaction && (
          <>
            <h2 className="font-medium">Accès demandés</h2>
            <p className="break-all text-sm text-muted-foreground">
              Connexion demandée par <strong>{state.interaction.clientId}</strong>.
            </p>
            <ul className="flex flex-col gap-3 text-sm">
              {[...new Set(state.interaction.scope.split(' ').filter(Boolean))].map((scope) => (
                <li key={scope} className="flex items-start gap-3">
                  <ArrowUpRight
                    className="mt-0.5 size-4 shrink-0 text-primary"
                    aria-hidden="true"
                  />
                  <span className="break-all">{scopeLabels[scope] ?? scope}</span>
                </li>
              ))}
            </ul>
            <Button
              size="lg"
              disabled={state.busy}
              onClick={() => void controller.consent('approve')}
            >
              Autoriser et continuer
            </Button>
            {(state.firstEnrollmentAvailable || securityExpiry) && (
              <Button
                variant="outline"
                disabled={state.busy}
                onClick={() => void controller.beginEnrollment()}
              >
                Ajouter une méthode de sécurité
              </Button>
            )}
            {!securityExpiry && !state.firstEnrollmentAvailable && (
              <Button
                variant="outline"
                disabled={state.busy}
                onClick={() => controller.beginSecurityStepUp()}
              >
                Confirmer mon identité pour gérer la sécurité
              </Button>
            )}
            {securityExpiry && (
              <Button
                variant="outline"
                disabled={state.busy}
                onClick={() => controller.beginFactors()}
              >
                Gérer mes méthodes de sécurité
              </Button>
            )}
          </>
        )}
        {active && state.stage !== 'factors' && (
          <>
            <Separator />
            <Button
              variant="ghost"
              disabled={state.busy}
              onClick={() => void controller.consent('deny')}
            >
              Annuler la connexion
            </Button>
          </>
        )}
        <p role="status" aria-live="polite" className="min-h-5 text-sm text-muted-foreground">
          {state.busy
            ? 'Vérification en cours…'
            : state.stage === 'leaving'
              ? 'Redirection en cours…'
              : ''}
        </p>
      </div>
    </AuthShell>
  );
}
