# GitHub sourcemaps extraction

Date: 2026-06-17

Source analysee: https://github.com/shaynlink

Surfaces publiques complementaires analysees:

- https://github.com/login
- https://github.com/github/docs
- https://github.com/orgs/community/discussions
- https://github.com/features/actions
- https://github.com/security
- https://github.com/enterprise

Artefacts telecharges localement:

- HTML: `/tmp/github_shaynlink.html`
- Bundles JS et sourcemaps: `/tmp/github-sourcemaps-analysis`
- Analyse complementaire: `/tmp/github-sourcemaps-extra`

## Cadre d'utilisation

Les sourcemaps publiques de GitHub contiennent du code source et des dependances. Pour `nvbes`, l'extraction utile doit rester conceptuelle: patterns, composants publics, decoupage applicatif et priorites produit. Ne pas copier de code applicatif GitHub dans le repo.

## Inventaire

- 44 bundles JavaScript charges par la page profile.
- 44 sourcemaps publiees et telechargeables.
- 845 sources referencees.
- Analyse complementaire:
  - 134 sourcemaps uniques sur les surfaces publiques additionnelles.
  - 4 157 sources referencees dans ce second lot.
  - 2 984 sources uniques nouvelles par rapport a la page profile.
- Les maps incluent `sourcesContent`.
- Principales familles observees:
  - `app/assets/modules/github/*`: petits comportements DOM et modules produit.
  - `@github/catalyst`: web components declaratifs.
  - `@primer/view-components` et `@primer/behaviors`: primitives UI, focus, dialogs, menus.
  - `@tanstack/react-router`: routage React.
  - `@github/hydro-analytics-client`: analytics centralisees.
  - elements publics GitHub: `clipboard-copy`, `relative-time`, `include-fragment`, `auto-complete`.
- Familles supplementaires observees:
  - `packages/code-view/*`: rendu de code, virtualisation, search results, deferred metadata.
  - `packages/verified-fetch`: fetch client contraint et verifie.
  - `packages/safe-html` et `packages/sanitize`: HTML verifie, Trusted Types.
  - `packages/error-with-retry` et `CodeViewError`: error boundaries par surface.
  - `packages/version-mismatch-detector`: detection de version frontend/backend obsolete.
  - `packages/query-builder-element`: recherche structuree avec scopes/autocomplete.
  - `packages/notifications-subscriptions-menu`: preferences de notification et API dediee.
  - `packages/use-alive` et live updates: abonnements temps reel gouvernes par visibilite.
  - `packages/cookie-consent` et `packages/consent-experience`: consentement localise par region.

## Patterns utiles pour nvbes

### 1. Runtime frontend en couches

GitHub separe nettement:

- un socle environnement/runtime (`environment`, `fetch-utils`, `safe-storage`, `html-safe-nonce`);
- des comportements transverses (`selector-observer`, `hotkey`, `remote-form`, `aria-live`);
- des composants DOM reutilisables (`clipboard-copy`, `relative-time`, `include-fragment`);
- des bundles par surface produit (`profile`, `sessions`, `notifications`).

Pour `nvbes`, le meilleur alignement est de renforcer `libs/ts/web-runtime` pour les concerns transverses, et garder les apps React focalisees sur les workflows produit.

### 2. Progressive enhancement ciblee

La page GitHub ne met pas tout dans React. Beaucoup de micro-interactions sont portees par des comportements DOM ou web components: copie presse-papier, relative time, include fragment, autocomplete, dialogs, hotkeys, navigation au clavier.

Pour `nvbes`, cela suggere:

- garder React pour les vues applicatives complexes;
- extraire les micro-interactions communes dans `libs/ts/web-ui` ou `libs/ts/web-runtime`;
- eviter de dupliquer `navigator.clipboard`, `aria-live`, storage tolerant et listeners DOM dans chaque app.

### 3. Primitives a fort ROI

Priorites exploitables sans copier de code GitHub:

- `ClipboardButton`: composant React partage avec fallback, etat "copie", annonce `aria-live`. Remplacer les copies locales dans `developer-web`.
- `RelativeTime`: composant partage pour dates de sessions, logs, tokens, webhooks et fichiers.
- `SafeStorage`: wrappers `localStorage`/`sessionStorage` tolerants aux exceptions, a utiliser dans les sessions Drive/Developer et le consentement.
- `LiveRegion` ou `Announcer`: primitive partagee pour toasts, operations async et erreurs formulaire.
- `CommandSearch`/combobox: une primitive basee sur shadcn/Radix pour recherche de commandes dans les portails, pas un web component Catalyst.
- `RemoteDataFragment`: abstraction React/TanStack Query pour zones chargees a la demande, inspiree du pattern include-fragment mais idiomatique React.

### 4. Sessions et securite UX

Les maps exposent des modules dedies a `session-resume`, `stale-session`, `webauthn-support`, `two-factor` et `clear-data-on-logout`.

Pour `identity-web` et `developer-web`, les idees utiles sont:

- detection centralisee de session expiree/stale dans `web-runtime`;
- reprise de session explicite plutot qu'erreurs disperses;
- nettoyage storage au logout;
- detection de support WebAuthn exposee comme capability typed, pas testee dans chaque page.

### 5. Analytics gouvernees

GitHub isole tracking, contexte marketing, request context, pageview et client id. `nvbes` a deja un runtime analytics/consent; la lecon est de garder un seul pipeline consent-aware.

Priorites:

- conserver `libs/ts/web-runtime/src/analytics.*` comme seule entree analytics;
- interdire l'envoi direct depuis les pages;
- enrichir les evenements avec contexte applicatif stable cote runtime;
- garder le nettoyage des cles analytics au retrait du consentement.

### 6. Fetch verifie et contractuel

GitHub expose une couche `verified-fetch` qui force les appels applicatifs a rester sur un chemin relatif ou meme origine, ajoute un header de verification, et propose des helpers JSON. Le pattern est plus strict qu'un simple wrapper `fetch`: il encode une politique de securite dans l'API cliente.

Pour `nvbes`, creer `libs/ts/web-runtime/src/verified-fetch.ts`:

- refuser le cross-origin par defaut;
- ajouter un header `Nvbes-Verified-Fetch` ou equivalent;
- inclure CSRF/credentials de maniere uniforme;
- normaliser les erreurs HTTP;
- proposer `verifiedFetchJson<T>()` avec validation Zod quand le schema existe;
- interdire les appels directs a `fetch` dans les apps sensibles via lint ou revue.

### 7. Safe HTML et Trusted Types

Les sourcemaps contiennent `VerifiedHTML`, `SafeHTMLString`, `TrustedHTML`, une politique Trusted Types et des composants qui rendent explicitement du HTML verifie. C'est un pattern important pour eviter le `dangerouslySetInnerHTML` non gouverne.

Pour `nvbes`, utile sur:

- previews d'emails transactionnels;
- markdown/documentation;
- logs ou payloads enrichis;
- consent/OAuth previews;
- contenu importe dans Drive.

Priorites:

- definir un type opaque `SafeHtml`;
- creer un composant `VerifiedHtml`;
- centraliser la sanitization;
- refuser `dangerouslySetInnerHTML` hors composant approuve;
- ajouter plus tard CSP `trusted-types`.

### 8. Detection des caracteres Unicode invisibles

GitHub signale les caracteres Unicode caches dans les fichiers. Le risque est reel pour les secrets, redirect URIs, scopes, commandes, webhooks et snippets config: un caractere bidi ou invisible peut modifier la lecture humaine sans changer la valeur machine.

Pour `nvbes`, ajouter une primitive partagee:

- detection des caracteres bidi/invisibles;
- affichage d'une banniere d'avertissement;
- option "reveler les caracteres";
- usage cible dans `developer-web`: token debugger, secrets, scopes, redirect URIs, webhook payloads.

### 9. Error boundaries par surface

GitHub a des erreurs dediees au code view (`CodeViewError`, `CodeViewBlobLayoutErrorBoundary`) et un composant `ErrorWithRetry`. La logique d'erreur est au niveau de la surface produit, pas seulement une page blanche globale.

Pour `nvbes`, creer dans `web-runtime`:

- `FeatureBoundary`;
- `ErrorWithRetry`;
- contexte `app`, `surface`, `route`, `operation`;
- reporting automatique via le module d'observabilite frontend;
- integration TanStack Query reset/retry.

### 10. Version mismatch detector

Le second lot expose un detecteur de mismatch version avec rate limit local. Le pattern evite les boucles de reload et les erreurs bizarres apres un deploy.

Pour `nvbes`:

- injecter un `buildId` frontend;
- exposer un `releaseId` backend;
- comparer sur erreurs API ou endpoint health leger;
- afficher un toast "Nouvelle version disponible";
- rate limiter le reload via `safe-storage`.

### 11. Virtualisation et chargement differe des gros contenus

Les modules `code-view` utilisent `@tanstack/react-virtual`, des contextes `DeferredAST`, `DeferredMetadata`, des resultats de recherche et des marqueurs de scroll.

Pour `nvbes`, appliquer aux surfaces a gros volume:

- `drive-web`: preview texte, CSV, logs de scan, gros metadata panels;
- `developer-web`: logs API, traces, audit events, webhook deliveries;
- `identity-web`: historique de sessions ou consentements si la liste grossit.

Pattern recommande:

- rendre le shell rapidement;
- virtualiser les lignes/items;
- charger les metadonnees lourdes via TanStack Query;
- afficher des scroll marks pour erreurs, matches de recherche ou warnings.

### 12. Query builder et item pickers

GitHub a un query builder avec scopes, autocomplete, events, items lazy et fallback d'erreur. C'est directement transposable au portail developpeur.

Pour `developer-web`:

- recherche logs avec `client:`, `status:`, `route:`, `since:`, `trace:`;
- recherche webhooks avec `event:`, `delivery:`, `status:`;
- recherche OAuth apps avec `owner:`, `scope:`, `redirect:`;
- item picker lazy pour labels, apps, clients ou roles.

Implementation `nvbes`: utiliser shadcn/Radix/TanStack, pas le web component GitHub.

### 13. Live updates gouvernes par visibilite

Les modules `use-alive` et `use-checks-live-updates` montrent un pattern d'abonnement live qui tient compte de la visibilite de la page. C'est superieur a un polling fixe.

Pour `nvbes`:

- SSE ou polling intelligent pour jobs, scans, webhooks et logs;
- pause quand l'onglet est cache;
- backoff sur erreurs;
- indicateur stale;
- reprise propre au focus.

### 14. Consentement par region

GitHub separe le consentement par experience/langue/region. `nvbes` vise d'abord UE, mais la structure peut deja eviter le hardcode d'un seul texte.

Pour `nvbes`:

- garder les categories et vendors dans `web-runtime`;
- ajouter une couche `policyRegion`;
- conserver les textes/legal copies hors logique de stockage;
- preparer les futures variantes sans detourner le modele actuel.

## Ce qu'il ne faut pas importer

- Ne pas adopter `@github/catalyst` comme framework principal: `nvbes` est deja React + shadcn/Radix/TanStack.
- Ne pas copier les modules `app/assets/modules/github/*`.
- Ne pas ajouter Primer comme design system concurrent au registry shadcn/local.
- Ne pas reproduire le modele GitHub de bundles globaux si une primitive React partagee suffit.
- Ne pas reprendre `packages/code-view` comme architecture complete: extraire seulement les patterns virtualisation/deferred/error.
- Ne pas introduire un query builder web component si une implementation React/shadcn couvre le besoin.

## Backlog recommande

1. Ajouter `verified-fetch` dans `libs/ts/web-runtime` et migrer les clients web sensibles.
2. Ajouter `safe-storage` dans `libs/ts/web-runtime` et migrer les stockages session/consentement.
3. Creer `FeatureBoundary` et `ErrorWithRetry` partages, avec reporting d'erreurs.
4. Creer `SafeHtml` / `VerifiedHtml` et interdire le HTML non verifie hors composant approuve.
5. Ajouter un detecteur de caracteres Unicode invisibles pour `developer-web`.
6. Ajouter un detecteur de mismatch version frontend/backend avec reload controle.
7. Creer dans `libs/ts/web-ui` un `ClipboardButton` partage et migrer `developer-web` (`QuickstartPage`, `SecretsPage`, `PortalAppsPage`).
8. Ajouter un composant `RelativeTime` partage, avec dates absolues en title pour auditabilite.
9. Centraliser la detection session stale dans le client HTTP/runtime au lieu de la traiter page par page.
10. Introduire une primitive query builder/command search shadcn pour `developer-web`.
11. Virtualiser les listes/gros contenus dans `drive-web` et `developer-web` quand les volumes depassent le rendu confortable.
12. Introduire un runtime live updates pause-on-hidden pour jobs, scans, webhooks et logs.

## Sources publiques observees

- Page analysee: https://github.com/shaynlink
- Pages additionnelles: https://github.com/login, https://github.com/github/docs, https://github.com/orgs/community/discussions, https://github.com/features/actions, https://github.com/security, https://github.com/enterprise
- Host assets: https://github.githubassets.com/assets/
- Examples de maps telechargees:
  - `profile-5f5415cae47dbbae-98d499b0da3638de.js.map`
  - `behaviors-86908fdfd21723bf-701999e9d071d81f.js.map`
  - `github-elements-1ba2f4478695d788-c21a71f7b7a22b8c.js.map`
  - `react-core-e5b3efd7778d62d3-7b5790d2512eb719.js.map`
  - `code-view-451dc5d4033224d9-ad2b9803b118815c.js.map`
  - `settings-a7befc88bc6ac60f-8c177f92070ff4be.js.map`
  - `discussions-a8b2539a9270b336-f36c5fb6ee539c55.js.map`
  - `landing-pages-2232bf710f9db89e-f6f2855c340fa606.js.map`
