# nvbes Identity Service

Authorization Server OAuth 2.1 / OpenID Connect et service d'identité de nvbes.

Identity est la source d'autorité pour les identités, les facteurs
d'authentification, les sessions et l'émission des tokens. Il ne porte pas les
profils, préférences, consentements de confidentialité ni demandes de fermeture
du produit Account.

## Responsabilités

- Universal Login hébergé par `identity-web`;
- authentification par mot de passe, passkeys/WebAuthn et MFA;
- vérification et récupération des adresses de connexion;
- sessions navigateur et gestion des appareils;
- Authorization Code avec PAR et PKCE `S256`;
- refresh tokens rotatifs, révocation, introspection, JWKS et UserInfo;
- clients OAuth humains et machine-to-machine;
- fédération OIDC/SAML, SCIM et décisions d'autorisation Identity;
- listener gRPC interne d'introspection.

Chaque resource server valide localement la signature, l'issuer et sa propre
audience. Par exemple, `account-service` n'accepte que
`aud = nvbes-account-service`.

## Endpoints publics principaux

### Authentification

- `POST /api/v1/auth/register`
- `POST /api/v1/auth/challenge/identifier`
- `POST /api/v1/auth/challenge/pwd`
- `POST /api/v1/auth/challenge/mfa`
- `POST /api/v1/auth/logout`
- `POST /api/v1/auth/verify-email`
- `POST /api/v1/auth/password/forgot`
- `POST /api/v1/auth/password/reset`
- `GET /api/v1/auth/sessions`
- `DELETE /api/v1/auth/sessions/{id}`
- `GET /api/v1/auth/mfa/factors`

### OAuth et OIDC

- `POST /oauth/par`
- `GET /oauth/authorize`
- `GET /oauth/hosted-login/{stateId}`
- `POST /oauth/hosted-login/{stateId}/authorize`
- `POST /oauth/hosted-login/{stateId}/consent`
- `POST /oauth/token`
- `POST /oauth/introspect`
- `POST /oauth/revoke`
- `GET /oauth/userinfo`
- `GET /.well-known/openid-configuration`
- `GET /.well-known/jwks.json`

Le navigateur commence par PAR, conserve `state`, `nonce` et le verifier PKCE
en `sessionStorage`, puis suit la redirection HTTP 303 vers `identity-web`.
Aucun secret client n'est embarqué dans les applications web publiques.

## Configuration locale

Les scripts locaux adaptent les variables dédiées au runtime commun :

```dotenv
NVBES_IDENTITY_DATABASE_URL=postgres://postgres:postgres@localhost:5432/nvbes_identity
NVBES_IDENTITY_SERVICE_BASE_URL=http://localhost:4000
NVBES_IDENTITY_SERVICE_PORT=4000
NVBES_IDENTITY_WEB_BASE_URL=http://localhost:3000
NVBES_IDENTITY_GRPC_PORT=4010
```

## Lancement et validation

```bash
pnpm dev:identity-service
pnpm dev:identity-web
pnpm exec nx run identity-service:check
pnpm exec nx run identity-service:test-contract
```

Les migrations Identity sont exclusivement dans
`apps/identity-service/migrations`. Les données Account utilisent une base et
un historique de migrations distincts.
