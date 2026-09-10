# @nvbes/identity-sdk-web

SDK navigateur pour les parcours nvbes Identity et les clients OAuth publics.

## Principes de sécurité

- Authorization Code avec PAR et PKCE S256 obligatoire.
- Aucun secret OAuth dans le navigateur.
- La transaction temporaire `state`/verifier/nonce vit uniquement dans `sessionStorage`.
- Les access tokens restent sous la responsabilité de l’application et ne sont jamais persistés
  par le SDK.
- Les appels Identity interactifs utilisent les cookies HttpOnly Identity. L’échange de code OAuth
  utilise `credentials: "omit"`.

## Client OAuth public

La primitive `dpopFetch` refuse les erreurs de signature et de transport sans
nouvelle tentative implicite. Elle interdit les redirections et omet les cookies.
Les clés privées WebCrypto sont non exportables. Les helpers de nonce exigent
désormais l'URL du serveur en dernier argument ; le cache est isolé par origine,
limité à 32 entrées et 1 024 caractères par nonce. Les preuves excluent query et
fragment de `htu`. Le mode worker conserve le contrat de son fournisseur crypto.

Le parcours OAuth active DPoP par défaut. Chaque transaction conserve sa propre
clé privée non exportable dans IndexedDB (15 minutes, 32 entrées maximum).
`sessionStorage` ne contient que sa référence et sa liaison issuer/client/redirect.
Le SDK refuse une clé absente au callback et un token Bearer pour une transaction
DPoP. Après échange, la clé quitte IndexedDB et revient en mémoire dans
`result.dpopKey` ; les appels API doivent la passer explicitement à
`dpopFetch(url, { key: result.dpopKey, accessToken: result.accessToken })`.
L'application conserve ce résultat en mémoire, jamais en JSON ou localStorage.

Un seul parcours est autorisé à la fois dans un même stockage de transaction.
Une deuxième autorisation ne remplace pas la première. Une panne IndexedDB
interrompt le démarrage sans fallback Bearer. `dpop: false` désactive explicitement
la liaison pour un client dont le registre le permet. `dpopStore` permet d'injecter
un stockage respectant le contrat `DpopTransactionStore` pour les tests.
La persistance de clé a été vérifiée après rechargement dans Chromium ; les
échanges OAuth de cette preuve utilisent des réponses simulées. Refresh DPoP,
validation complète OIDC et parcours contre les vrais services restent à valider.

```typescript
import { NvbesIdentityWeb } from '@nvbes/identity-sdk-web';

const identity = new NvbesIdentityWeb({
  baseUrl: 'https://identity.nvbes.eu',
  clientId: 'account-web',
  redirectUri: 'https://account.nvbes.eu/oauth/callback',
  resource: 'https://account-api.example', // URL exacte du registre OAuth Identity
});

await identity.redirectToLogin({
  scope: 'openid account:read account:write',
  returnTo: '/profile',
});
```

`redirectToLogin` pousse d’abord la requête sur `/oauth/par`, crée une transaction PKCE à usage
unique, puis navigue vers `/oauth/authorize` avec seulement `client_id` et `request_uri`.
`resource` remplace l'ancien paramètre SDK `audience` : l'audience du jeton est
dérivée du registre côté serveur. `openid` est obligatoire ; les scopes métier
doivent être enregistrés pour cette ressource. PAR refuse les redirections HTTP
et omet cookies et cache. L'issuer configuré doit être une origine HTTPS (HTTP
loopback uniquement en développement).

Dans la route de callback :

```typescript
const params = new URLSearchParams(window.location.search);
const code = params.get('code');
const state = params.get('state');

if (!code || !state) {
  throw new Error('Réponse OAuth incomplète.');
}

const result = await identity.exchangeAuthorizationCode({ code, state });
// Conserver result.accessToken en mémoire seulement.
window.location.replace(result.returnTo);
```

Le SDK valide `state`, l’âge de la transaction et le verifier PKCE avant l’échange. Il supprime la
transaction après succès. Un éventuel `idToken` reste opaque : une application qui consomme ses
claims doit le valider selon OpenID Connect avec l’issuer et le JWKS Identity.

## Session Identity

Les méthodes `getCurrentUser`, `logout`, MFA, WebAuthn et step-up s’adressent directement au
domaine Identity et utilisent sa session HttpOnly. Elles sont destinées au site Identity, pas aux
Resource Servers comme Account.

## Détection d’environnement

```typescript
import { parseUserAgent } from '@nvbes/identity-sdk-web';

const environment = parseUserAgent(navigator.userAgent, {
  vendor: navigator.vendor,
});
```

Les informations détaillées retournées par `parseUserAgent()` ne sont pas ajoutées
automatiquement au profil de confiance d’appareil.
