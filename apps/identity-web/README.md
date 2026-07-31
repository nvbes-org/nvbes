# nvbes Identity Web

Site React/Vite autonome du domaine Identity.

## Responsabilités

- connexion, inscription et récupération de mot de passe ;
- Universal Login et consentement OAuth ;
- vérification des adresses e-mail ;
- mot de passe, MFA, passkeys, clés de sécurité et codes de récupération ;
- appareils, sessions et applications autorisées.

Les profils, préférences produit, notifications et fonctions de confidentialité Account ne
font pas partie de cette application.

## Routes

- publiques : `/login`, `/register`, `/verify`, `/verify-email`, `/verify-result`,
  `/forgot-password`, `/reset-password` ;
- authentifiées : `/security`, `/security/password`, `/email-addresses`, `/sessions`,
  `/linked-apps`, `/mfa` et `/mfa/*`.

L'application possède son propre shell, son propre routeur et sa propre configuration Vite.

## Développement

```bash
pnpm nx run identity-web:dev
pnpm nx run identity-web:typecheck
pnpm nx run identity-web:test
```

Le proxy Vite utilise `VITE_IDENTITY_SERVICE_BASE_URL` (par défaut
`http://localhost:4000`). Le port du site se configure avec
`VITE_IDENTITY_WEB_PORT` (par défaut `3000`).

## Contrats principaux

- `/auth/*` pour les challenges, sessions, facteurs et adresses e-mail ;
- `/oauth/*` pour PAR, authorization-code, Universal Login et consentement ;
- `/api/*` pour les endpoints Identity encore exposés sous ce préfixe.
