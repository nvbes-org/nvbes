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

`listHostedPasskeys(transport, sessionCsrf)` retourne les métadonnées des clés
actives (`id`, `label`, `createdAt`, `lastUsedAt`), sans données cryptographiques.
`renameHostedPasskey(transport, sessionCsrf, id, label)` et
`revokeHostedPasskey(transport, sessionCsrf, id)` exigent une confirmation serveur.
Le serveur exige une authentification forte récente pour ces mutations et refuse
la suppression du dernier facteur fort avec `HostedIdentityError.status === 409`.
Révoquer une clé ferme toutes les sessions qui l'ont utilisée, y compris celle
du demandeur le cas échéant : l'interface doit alors engager une reconnexion.
Les erreurs ne sont pas réessayées automatiquement. Ces fonctions utilisent les
routes actives et sont également disponibles depuis `./oauth`.

`NvbesIdentityWeb.logout(sessionCsrfToken)` exige désormais cette preuve explicite
et confirme réellement le succès serveur. Il ne peut pas être appelé depuis
Account pour envoyer un cookie à Identity. Le parcours de déconnexion intersites
reste à livrer. Les méthodes de récupération et `stepUp` générique du wrapper
restent à migrer ; elles ne constituent pas un contrat aligné sur les routes actives.

### TOTP hébergé

`NvbesIdentityWeb.startTotpEnrollment(sessionCsrf)` retourne `factorId`,
`secretBase32`, `provisioningUri` et `expiresAt`. Le secret et son URI sont
sensibles : les afficher uniquement dans Identity, sans journalisation ni
stockage navigateur. Le SDK valide le profil émis par nvbes et la correspondance
entre URI et secret. La transaction expire après cinq minutes côté serveur.

`confirmTotpEnrollment(sessionCsrf, factorId, code)` active le facteur et retourne
l'échéance du step-up accordé. `stepUpTotp(sessionCsrf, code)` vérifie un code
d'un facteur actif et retourne son échéance. Les codes contiennent exactement
six chiffres ASCII ; un code consommé ne peut pas être réutilisé. Les requêtes
exigent cookies et CSRF sur l'origine Identity, sans Bearer ni retry implicite.
Les erreurs HTTP, notamment 400 et 429, remontent via `HostedIdentityError`.

Les fonctions autonomes `startHostedTotpEnrollment`, `confirmHostedTotpEnrollment`
et `stepUpHostedTotp` sont exportées depuis le package et `./oauth`.
Les anciens `setupTotp` et `confirmTotp` (méthodes et fonctions) ont été supprimés :
ils appelaient `/auth/mfa/totp/*` archivé. Migrer explicitement vers les nouvelles
signatures ; le label et le Bearer des anciennes signatures ne sont pas acceptés.
Les types générés historiques de `identity-sdk-core` restent distincts de ce contrat.

`apps/identity-service/tests/runtime-browser-totp.mjs` vérifie les trois méthodes
avec Chromium et les services HTTPS réels : provisioning, confirmation, step-up,
refus des codes consommés, puis échange OIDC et accès Account avec DPoP.
Son authentificateur synthétique utilise WebCrypto ; aucune application mobile
TOTP réelle n'est testée. Lancer une fixture neuve avec
`pnpm nx run identity-service:test:https-browser-fixture`, puis appeler
`verifyBrowserTotp(browser, clientOrigin)` dans Playwright. La fixture est isolée
et doit être arrêtée après le test. Ce parcours interactif ne tourne pas en CI.

`listTotpFactors(sessionCsrf)` retourne zéro ou un facteur actif (`id`, `createdAt`).
`revokeTotpFactor(sessionCsrf, id)` exige une preuve forte récente et une passkey
active en remplacement ; le serveur refuse le dernier facteur avec HTTP 409.
Le succès confirme aussi la révocation de **toutes les sessions du compte** :
engager une nouvelle autorisation OAuth. Le secret chiffré du facteur est effacé.
Les exports autonomes sont `listHostedTotpFactors` et `revokeHostedTotpFactor`.
Les anciennes méthodes génériques `listMfaFactors`/`removeMfaFactor` restent
historiques ; utiliser ces méthodes dédiées pour le TOTP actif.
Le mode `verifyBrowserTotp(browser, clientOrigin, true)` sur une fixture neuve
teste la gestion : liste, refus du dernier facteur, ajout d'une passkey virtuelle,
révocation TOTP puis refus de la session et du jeton Account déjà émis.

### Migration des méthodes WebAuthn

Les anciennes méthodes WebAuthn et leurs exports autonomes ont été supprimés :
ils appelaient les endpoints archivés `/auth/mfa/webauthn/*`. Aucun consommateur
actif du monorepo n'utilisait ces méthodes. Pour un consommateur externe de l'alpha,
ce changement nécessite une adaptation explicite :

| Ancien appel                                               | Appel actif sur `NvbesIdentityWeb`                                   |
| ---------------------------------------------------------- | -------------------------------------------------------------------- |
| `startWebAuthnRegistration` + `finishWebAuthnRegistration` | `registerPasskey(sessionCsrf, label, options?)`                      |
| `registerWebAuthnCredential`                               | `registerPasskey(sessionCsrf, label, options?)`                      |
| `startWebAuthnAuthentication` + `completeWebAuthnStepUp`   | `stepUpPasskey(sessionCsrf, options?)`                               |
| Connexion découvrable                                      | `loginPasskey(interaction, options?)`                                |
| Gestion des clés                                           | `listPasskeys`, `renamePasskey`, `revokePasskey` avec CSRF explicite |

La politique de l'authentificateur vient du serveur. Les méthodes ne prennent
plus de Bearer token ni de `kind` local ; elles exigent l'origine Identity et
les preuves de session/interaction actives. Les fonctions `*HostedPasskey(s)`
restent disponibles pour les interfaces qui préfèrent des fonctions autonomes.

## Compatibilité des clés non découvrables

`loginHostedSecurityKey(transport, interaction, { email, password }, options?)`
et `NvbesIdentityWeb.loginWithSecurityKey(interaction, credentials, options?)`
enchaînent le login mot de passe et un step-up WebAuthn avec vérification
utilisateur obligatoire. Le résultat contient `interaction` et `stepUpExpiresAt`.
Passer ensuite cette interaction à `completeHostedConsent`.

La liste des credentials est obtenue après authentification. Ce choix évite
l'exposition de `allowCredentials` sur le seul email, dont les risques de
corrélation et d'énumération sont décrits par le
[W3C](https://www.w3.org/TR/webauthn-3/#sctn-credential-id-privacy).
La connexion sans mot de passe reste `loginPasskey` avec une clé découvrable.
Ce parcours de compatibilité exige un mot de passe ; il ne livre pas une
connexion passwordless par email avec clé non découvrable.

Si la cérémonie échoue ou est annulée, le SDK tente de révoquer la session créée
par le login et lève `HostedSecurityKeyLoginError`. Sa propriété
`sessionRevocationConfirmed` distingue un logout confirmé d'une panne de cleanup.
Aucune interaction de succès ni fallback mot de passe n'est retourné. En cas
de panne de cleanup, l'interface ne doit pas affirmer que la session est fermée.
Ces opérations restent des requêtes distinctes : cette orchestration SDK ne
remplace pas une politique MFA obligatoire appliquée côté serveur.

Le test `runtime-browser-security-key.mjs` expose
`verifyBrowserSecurityKey(browser, clientOrigin)` sur une fixture HTTPS neuve.
Il vérifie l'inscription d'une clé CTAP2 USB virtuelle sans capacité résidente,
la fermeture de session après annulation, puis la connexion et l'accès Account.
Il ne prouve pas la compatibilité d'un modèle de clé physique ou de tous les OS.

## Détection d’environnement

```typescript
import { parseUserAgent } from '@nvbes/identity-sdk-web';

const environment = parseUserAgent(navigator.userAgent, {
  vendor: navigator.vendor,
});
```

Les informations détaillées retournées par `parseUserAgent()` ne sont pas ajoutées
automatiquement au profil de confiance d’appareil.
