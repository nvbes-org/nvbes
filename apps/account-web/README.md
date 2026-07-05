# nvbes Identity Web

Portail React/TypeScript pour nvbes Identity.

## Fonctionnalites

- Connexion par flux challenge `identifier -> password -> MFA`.
- Universal Login hébergé pour les flux OAuth authorization-code + PKCE.
- Inscription avec detection/selection de region supportee.
- Verification email, renvoi et changement d'adresse.
- Gestion compte, sessions, MFA TOTP, passkeys, cles de securite et codes de recuperation.
- Workspaces, billing via service dedie, consentements legaux et vues operationnelles securite.
- Activation OAuth Device Flow via `/oauth/device/*`.

## Runtime

- Router: TanStack Router dans `src/identity.router.tsx`.
- Server state: TanStack Query via `*.queries.ts`.
- Mutations et orchestration async: fonctions TanStack-friendly via `*.functions.ts`.
- HTTP valide: `@nvbes/http-client`, `@nvbes/identity-client` et `@nvbes/identity-sdk-web`.

Les composants ne devraient pas ajouter de nouveaux `fetch` directs. Ajouter plutot:

- `*.api.ts` pour les appels HTTP valides par Zod;
- `*.functions.ts` pour l'orchestration async;
- `*.queries.ts` pour les query/mutation keys.

## Structure

```text
src/
├── main.tsx
├── App.tsx
├── identity.router.tsx
├── identity.auth.api.ts
├── identity.auth.functions.ts
├── identity.auth.queries.ts
├── identity.email-verification.ts
├── account.queries.ts
├── components/
├── hooks/
├── lib/
└── pages/
```

## Contrats backend utilises

- Auth web:
  - `POST /api/v1/auth/challenge/identifier`
  - `POST /api/v1/auth/challenge/pwd`
  - `POST /api/v1/auth/challenge/mfa`
  - `POST /api/v1/auth/logout`
- Compte:
  - `GET /api/v1/auth/me`
  - `GET /api/v1/auth/accounts`
  - `GET /api/v1/auth/sessions`
  - `DELETE /api/v1/auth/sessions/{sessionId}`
  - `POST /api/v1/auth/sessions/revoke-others`
- MFA:
  - `GET /api/v1/auth/mfa/factors`
  - `POST /api/v1/auth/mfa/totp/setup`
  - `POST /api/v1/auth/mfa/totp/confirm`
  - `POST /api/v1/auth/mfa/webauthn/register/start`
  - `POST /api/v1/auth/mfa/webauthn/register/finish`
- Billing:
  - les vues Billing peuvent rester dans Identity Web;
  - les lectures/actions Billing doivent passer par `billing-service` ou le gateway, pas par l'API Identity locale.
- OAuth Device Flow:
  - `POST /oauth/device/verify`
  - `POST /oauth/device/approve`
  - `POST /oauth/device/deny`

## Demarrage

```bash
pnpm --dir apps/account-web dev
```

Le proxy Vite redirige `/api` et `/oauth` vers l'Account Service.
