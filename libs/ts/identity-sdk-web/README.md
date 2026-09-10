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
La [preuve HTTPS Chromium](../../../docs/architecture/identity-browser-protocol-proof.md)
vérifie les échanges réels avec Identity, Account et Billing, sur deux origines
clientes, y compris la persistance IndexedDB et la révocation après logout.

Le callback vérifie l'ID token avec `jose` avant de retourner les jetons : signature
RS256, type JWT, issuer, audience client unique, `azp` éventuel, nonce, dates et
liaison `at_hash`. Le résultat `identity` contient le sujet et le contexte vérifiés.
Les clés viennent uniquement de `/oauth/jwks` sur l'issuer configuré, sans cookies
ni redirections ; réponse limitée à 64 Kio et 32 clés, délai de dix secondes.
Le SDK accepte le profil du service nvbes, pas tous les profils OIDC tiers.
Le refresh d'une session issue du callback revérifie l'ID token et refuse un
changement de sujet ou d'heure d'authentification. Construire `OAuthSession` avec
le résultat du callback, jamais des claims décodés sans validation.

## Renouvellement en mémoire

Après `exchangeAuthorizationCode`, construire un seul `OAuthSession` par famille
de jetons. Sa méthode `refresh()` mutualise les appels simultanés, remplace le
refresh token après chaque rotation et réutilise la même clé DPoP. Le client doit
être enregistré avec refresh autorisé et demander `offline_access` à l'autorisation.

```typescript
import { OAuthSession } from '@nvbes/identity-sdk-web';

const session = new OAuthSession({ baseUrl, clientId }, tokens);
const rotated = await session.refresh();
// Utiliser rotated.accessToken et rotated.dpopKey pour la prochaine requête API.
```

Les requêtes ont un délai de dix secondes, sans cookies, redirections ou nouvelle
tentative automatique. Toute erreur de rotation invalide l'état local : même si
la réponse est perdue, le serveur peut avoir consommé l'ancien secret. Une
reconnexion est alors nécessaire. `clear()` efface seulement l'état local et ne
remplace pas le logout serveur. Un résultat tardif ne réactive pas une session
effacée. Ne pas créer plusieurs instances à partir du même refresh token.

La cible `identity-service:test:resource-runtimes` traverse le vrai SDK TypeScript,
Identity, Account et Billing : PAR, code, rotations, accès API et refus après
logout. Elle utilise Node et un stockage de clé injecté ; la preuve Chromium
complète ces assertions avec CORS, cookies intersites et IndexedDB sous HTTPS.

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
transaction après succès. Il vérifie aussi l'ID token et retourne les claims
validés dans `identity` ; ne pas utiliser un simple décodage du JWT comme preuve.

## Session Identity

Les fonctions `loadHostedAuthorization`, `parseHostedInteraction`,
`loginHostedPassword`, `completeHostedConsent` et `logoutHostedSession` suivent
les routes `/oauth/*` actives. Elles sont exportées depuis le package et `./oauth`.
Chaque requête exige que `location.origin` corresponde à `baseUrl`, utilise
`credentials: 'same-origin'`, refuse les redirections automatiques et borne la
réponse à 64 Kio et dix secondes, sans retry. Le mot de passe n'est pas persisté.

Après login, utiliser l'interaction retournée : sa preuve CSRF a changé.
`completeHostedConsent` retourne l'URL validée par Identity ; l'interface peut
ensuite appeler `location.assign(destination)`. La preuve CSRF de session est
distincte et doit être passée à `logoutHostedSession`.

`registerHostedPasskey(transport, sessionCsrf, label)` enregistre une clé avec
les options du serveur et retourne son identifiant. `loginHostedPasskey(transport,
interaction)` effectue une connexion découvrable et retourne les nouvelles preuves
CSRF. `stepUpHostedPasskey(transport, sessionCsrf)` retourne l'échéance confirmée
par Identity. Les trois fonctions utilisent les routes WebAuthn `/oauth/*`,
exigent le RP de l'hôte Identity et `userVerification: required`, et acceptent
un dernier argument avec `signal`/`timeoutMs` pour la cérémonie native.
Une annulation n'envoie pas de requête finish et ne déclenche aucun fallback.
Ces fonctions sont exportées depuis le package et `./oauth` ; elles appartiennent
exclusivement à l'interface hébergée sur l'origine Identity.

`NvbesIdentityWeb.logout(sessionCsrfToken)` exige désormais cette preuve explicite
et confirme réellement le succès serveur. Il ne peut pas être appelé depuis
Account pour envoyer un cookie à Identity. Le parcours de déconnexion intersites
reste à livrer. Les anciennes méthodes MFA/WebAuthn du wrapper ne constituent
pas encore un contrat aligné sur les routes actives ; leur migration reste ouverte.

## Détection d’environnement

```typescript
import { parseUserAgent } from '@nvbes/identity-sdk-web';

const environment = parseUserAgent(navigator.userAgent, {
  vendor: navigator.vendor,
});
```

Les informations détaillées retournées par `parseUserAgent()` ne sont pas ajoutées
automatiquement au profil de confiance d’appareil.
