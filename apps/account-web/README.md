# Account Web

Client public OAuth/OIDC indépendant du site Identity. Application générée avec
Nx React puis alignée sur Vite+, TanStack Router et le SDK OAuth actif. Tailwind
a été installé avant l'initialisation du registry officiel shadcn (Radix Nova).
Le registry partagé ne fournit pas encore les primitives Button/Alert ; celles-ci
proviennent du registry officiel, sans dépendance vers le code d'Identity Web.

## Configuration publique

Servir `/account-config.json` sur l'origine Account, sans cache ni redirection :

```json
{
  "identityOrigin": "https://identity.example.test",
  "accountApiOrigin": "https://account-api.example.test",
  "clientId": "account-web"
}
```

Ces valeurs sont des exemples, pas une configuration activée. Aucun secret client
n'est accepté. Les URL sont des origines HTTPS exactes (HTTP loopback autorisé
en développement). Enregistrer le client côté Identity avec la redirection exacte
`<origine-account-web>/oauth/callback`, la ressource Account et les scopes
`openid account:read`, ainsi que les origines CORS des services concernés.
Une configuration manquante ou invalide ferme le parcours.

Le serveur d'hébergement doit servir `index.html` pour `/` et `/oauth/callback`,
avec `Cache-Control: no-store`, `Referrer-Policy: no-referrer` et
`X-Content-Type-Options: nosniff`. La CSP doit limiter scripts/styles/fonts à
`self`, les connexions à `self` et aux deux origines configurées, refuser les
frames et les bases HTML. Le routage de production reste à configurer.

## Cycle de la page

La connexion pousse une requête PAR avec PKCE S256 et DPoP. Le SDK conserve
seulement la transaction temporaire dans sessionStorage et sa clé non exportable
dans IndexedDB pendant la redirection. Le callback vérifie state, rejette les
paramètres dupliqués, retire immédiatement le code de l'URL et vérifie l'ID token
via le SDK. Tout résultat d'échange incertain exige un nouveau parcours.

L'access token et sa clé restent en mémoire. Le profil doit correspondre au
subject OIDC vérifié. Aucun refresh/offline_access n'est demandé dans ce premier
parcours. Expiration, fermeture locale et sortie de page retirent les données
de l'interface et la référence aux jetons ; recharger exige une nouvelle
autorisation. « Fermer Account sur cette page » ne révoque pas le grant serveur
et n'est pas le logout intersites.

« Se déconnecter avec Identity » efface les données locales puis navigue vers
`/logout` sur l'origine Identity configurée. Une confirmation sur ce site révoque
la session serveur et invalide ses grants, y compris dans les autres clients.
L'annulation conserve la session Identity, mais ne restaure pas les jetons locaux
d'Account. Le retour RP standardisé et les notifications back-channel restent
des exigences ouvertes du lot C.

Au retour de l'onglet au premier plan, Account retire le profil affiché pendant
une nouvelle lecture authentifiée. L'API vérifie le grant et la session Identity
actuels ; un refus ou une panne ferme l'accès local sans réutiliser le profil
précédent. Les événements focus/visibilité concurrents partagent une seule lecture.
Il n'y a pas de polling périodique. Une page restée continuellement au premier
plan attend encore la notification proactive prévue par le lot C.

## Validation

```sh
pnpm dev:account-web
pnpm nx run-many -t typecheck,lint,test,build,format:check -p account-web
pnpm nx run identity-web:build
NVBES_IDENTITY_TEST_WEB_UI=1 NVBES_IDENTITY_TEST_ACCOUNT_WEB=1 \
  pnpm nx run identity-service:test:https-browser-fixture
```

La fixture sert les deux builds réels sur HTTPS, démarre les services et une base
isolée, utilise des identifiants synthétiques et se nettoie automatiquement.
Ajouter `NVBES_IDENTITY_TEST_REAL_FOCUS=1` pour vérifier le retour sur une seconde
fenêtre Account après logout : cette variante lance Chromium avec interface et
désactive l'émulation de focus de Playwright. Elle exige une session graphique.
Sans ce paramètre, le résultat du contrôle de retour est explicitement `not-run`.
Elle n'ouvre aucun service public et ne crée aucune ressource payante.
Le bootstrap local conjoint, les écritures du profil, Billing et la façade de
sécurité Account restent à raccorder aux contrats de leurs domaines.
