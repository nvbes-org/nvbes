import { useEffect, useRef, useSyncExternalStore } from 'react';
import { ArrowUpRight, LockKeyhole } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { AuthenticationForms } from './authorization.forms';
import type { AuthorizationController } from './authorization.controller';

const titles = {
  loading: 'Préparation de votre connexion',
  login: 'Votre espace commence ici.',
  'step-up': 'Confirmez que c’est vous.',
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
  const state = useSyncExternalStore(controller.subscribe, controller.snapshot);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, [state.stage]);
  const active =
    state.interaction !== null && !['closed', 'leaving', 'loading'].includes(state.stage);
  return (
    <div className="flex min-h-svh flex-col bg-background text-foreground">
      <header className="flex items-center justify-between px-6 py-7 sm:px-12">
        <span className="text-2xl font-semibold tracking-tighter">
          nvbes<span className="text-primary">.</span>
        </span>
        <span className="text-xs font-medium tracking-widest text-muted-foreground uppercase">
          Identity
        </span>
      </header>
      <main className="mx-auto flex w-full max-w-5xl flex-1 items-center px-6 py-10 sm:px-12">
        <section
          className="grid w-full gap-12 md:grid-cols-[1fr_1fr] md:gap-20"
          aria-busy={state.busy}
        >
          <div className="flex flex-col gap-5">
            <LockKeyhole className="size-7 text-primary" aria-hidden="true" />
            <h1
              ref={heading}
              tabIndex={-1}
              className="max-w-sm font-heading text-4xl leading-tight tracking-tight outline-none sm:text-5xl"
            >
              {titles[state.stage]}
            </h1>
            {state.interaction && (
              <p className="text-sm leading-relaxed text-muted-foreground">
                Connexion demandée par{' '}
                <strong className="break-all font-medium text-foreground">
                  {state.interaction.clientId}
                </strong>
                .
              </p>
            )}
            {state.stage === 'login' && (
              <p className="max-w-xs text-sm leading-relaxed text-muted-foreground">
                Utilisez votre compte nvbes pour continuer vers votre application.
              </p>
            )}
            {state.stage === 'consent' && (
              <p className="max-w-xs text-sm leading-relaxed text-muted-foreground">
                Vérifiez les accès demandés avant de continuer.
              </p>
            )}
          </div>
          <div className="flex flex-col gap-6">
            {state.error && (
              <Alert variant="destructive">
                <AlertDescription>{state.error}</AlertDescription>
              </Alert>
            )}
            {(state.stage === 'login' || state.stage === 'step-up') && (
              <AuthenticationForms controller={controller} state={state} />
            )}
            {state.stage === 'consent' && state.interaction && (
              <>
                <h2 className="font-medium">Accès demandés</h2>
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
              </>
            )}
            {active && (
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
        </section>
      </main>
      <footer className="px-6 py-7 text-xs text-muted-foreground sm:px-12">
        Un compte nvbes. Vos accès, votre choix.
      </footer>
    </div>
  );
}
