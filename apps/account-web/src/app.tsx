import { useState, useSyncExternalStore } from 'react';
import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router';
import { Button } from './components/ui/button';
import { Alert, AlertDescription } from './components/ui/alert';
import type { AccountController } from './account.controller';

export function App({ controller }: { controller: AccountController }) {
  const [router] = useState(() => {
    const root = createRootRoute({ component: Outlet, notFoundComponent: () => <MissingPage /> });
    const home = createRoute({
      getParentRoute: () => root,
      path: '/',
      component: () => <AccountPage controller={controller} />,
    });
    return createRouter({ routeTree: root.addChildren([home]) });
  });
  return <RouterProvider router={router} />;
}

function AccountPage({ controller }: { controller: AccountController }) {
  const state = useSyncExternalStore(controller.subscribe, controller.snapshot);
  return (
    <main className="mx-auto flex min-h-svh max-w-3xl flex-col px-6 py-10 sm:px-12">
      <header className="flex items-baseline justify-between border-b pb-6">
        <span className="text-2xl font-semibold tracking-tighter">nvbes.</span>
        <span className="text-sm text-muted-foreground">Account</span>
      </header>
      <section
        className="flex flex-1 flex-col justify-center gap-6 py-12"
        aria-busy={['loading', 'leaving'].includes(state.stage)}
      >
        <p className="text-sm font-medium text-muted-foreground">VOTRE ESPACE PERSONNEL</p>
        <h1 className="text-4xl font-semibold tracking-tight sm:text-5xl">
          {state.profile ? 'Votre profil.' : 'Un compte pour vous retrouver.'}
        </h1>
        {state.message && (
          <Alert role="status">
            <AlertDescription>{state.message}</AlertDescription>
          </Alert>
        )}
        {state.stage === 'loading' && <p role="status">Ouverture de votre compte…</p>}
        {state.stage === 'leaving' && <p role="status">Redirection vers Identity…</p>}
        {state.stage === 'ready' && (
          <>
            <p className="max-w-lg leading-relaxed text-muted-foreground">
              Connectez-vous avec nvbes Identity pour consulter votre profil.
            </p>
            <Button className="w-fit" size="lg" onClick={() => void controller.login()}>
              Se connecter avec Identity
            </Button>
          </>
        )}
        {state.profile && (
          <>
            <dl className="divide-y border-y">
              {[
                ['Nom affiché', state.profile.displayName],
                ['Prénom', state.profile.firstname],
                ['Nom', state.profile.lastname],
                ['Nom d’utilisateur', state.profile.username],
                ['Région', state.profile.region],
              ].map(([label, value]) => (
                <div key={label} className="grid gap-1 py-4 sm:grid-cols-2">
                  <dt className="text-muted-foreground">{label}</dt>
                  <dd className="break-words font-medium">{value || 'Non renseigné'}</dd>
                </div>
              ))}
            </dl>
            <Button variant="outline" className="w-fit" onClick={() => controller.close()}>
              Fermer Account sur cette page
            </Button>
            <Button className="w-fit" onClick={() => controller.logout()}>
              Se déconnecter avec Identity
            </Button>
          </>
        )}
        {['error', 'closed'].includes(state.stage) && (
          <Button asChild className="w-fit">
            <a href="/">Revenir à Account</a>
          </Button>
        )}
      </section>
      <footer className="border-t pt-6 text-sm text-muted-foreground">
        Votre profil est géré par nvbes Account.
      </footer>
    </main>
  );
}

function MissingPage() {
  return (
    <main className="mx-auto max-w-xl px-6 py-20">
      <h1 className="mb-6 text-3xl">Page introuvable</h1>
      <Button asChild>
        <a href="/">Ouvrir Account</a>
      </Button>
    </main>
  );
}
