# nvbes Account Web

Application React indépendante dédiée aux données du produit Account.

## Responsabilités

- profil ;
- préférences d’interface ;
- préférences de notifications ;
- consentements, export et fermeture du compte.

L’authentification, les adresses e-mail, les mots de passe, les sessions, les applications liées,
MFA et WebAuthn appartiennent à `identity-web`.

## Authentification

Account Web est un client OAuth public. Il utilise Authorization Code avec PAR et PKCE via
`@nvbes/identity-sdk-web`, conserve l’access token uniquement en mémoire, puis appelle Account avec
`@nvbes/account-client` et un bearer token. Aucun cookie Identity n’est envoyé au service Account.

Variables requises :

- `VITE_IDENTITY_SERVICE_BASE_URL`
- `VITE_IDENTITY_WEB_BASE_URL`
- `VITE_ACCOUNT_SERVICE_BASE_URL`
- `VITE_ACCOUNT_OAUTH_CLIENT_ID`
- `VITE_ACCOUNT_OAUTH_REDIRECT_URI`

## Routes

- `/` redirige vers `/profile`
- `/oauth/callback`
- `/profile`
- `/preferences`
- `/notifications`
- `/privacy`

## Développement

```bash
pnpm nx dev account-web
pnpm nx test account-web
pnpm nx typecheck account-web
pnpm nx lint account-web
pnpm nx build account-web
```
