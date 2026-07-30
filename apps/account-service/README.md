# nvbes Account Service

API Rust (Axum) servant de fondation IdP / Authorization Server multi-tenant pour les produits nvbes.

Le service porte:

- l'authentification utilisateur globale;
- les sessions Redis et tokens OAuth/OIDC;
- la gouvernance `tenant -> organization? -> workspace`;
- les memberships, workspaces, membres et une partie des decisions d'acces;
- l'introspection OAuth publique et l'introspection gRPC privée consommée par les autres services;
- les principals machine (`service accounts`) et les clients OAuth M2M rattachés a un workspace.

## Architecture

- **Framework** : Axum + Tonic + Tokio
- **Base de données** : PostgreSQL (sqlx)
- **Auth** : OAuth 2.1 / OIDC, JWT courts, refresh tokens opaques rotatifs stockes dans Redis
- **Billing** : service separe `billing-service`; Identity fournit l'authz et les vues web peuvent consommer Billing
- **Structure** : fichiers Rust plats en dot-notation, modules exposes via `#[path]`

## Structure du code

```
src/
├── main.rs
├── identity.app.rs
├── identity.http.routes.rs
├── identity.http.error.rs
├── identity.http.request.rs
├── identity.domains.auth.*
├── identity.domains.oauth.*
├── identity.domains.workspaces.*
├── identity.domains.members.*
├── identity.domains.federation.*
├── identity.domains.security.*
├── identity.domains.legal.*
└── identity.email.*
```

## Modele documentaire cible

- `user` global comme identite humaine principale
- `principal` comme identite technique canonique pour l'audit, l'ownership et les clients OAuth
- `tenant` obligatoire techniquement
- `organization` optionnelle
- `workspace` comme contexte produit
- access tokens JWT courts pour le trafic normal
- validation de signature locale puis introspection gRPC pour l'état dynamique des tokens

Le contrat documentaire de contexte d'acces est:

```text
AuthContext
  principal_id
  principal_kind
  user_id
  session_id
  tenant_id
  workspace_id optional
  client_id
  acr
  amr
  device_id optional
```

Pour les principals machine, `principal_id` porte l'identite du service account et `user_id` est absent.
Pour les principals humains, `principal_id` et `user_id` sont alignes sauf delegation explicite.

## Endpoints principaux

### Auth classique
- `POST /api/v1/auth/register`
- `POST /api/v1/auth/challenge/identifier`
- `POST /api/v1/auth/challenge/pwd`
- `POST /api/v1/auth/challenge/mfa`
- `POST /api/v1/auth/challenge/webauthn/start`
- `POST /api/v1/auth/logout`
- `POST /api/v1/auth/verify-email`
- `POST /api/v1/auth/verify-email/resend`
- `POST /api/v1/auth/verify-email/change`
- `GET  /api/v1/auth/region`
- `GET  /api/v1/auth/regions`
- `POST /api/v1/auth/password/forgot`
- `POST /api/v1/auth/password/reset`
- `POST /api/v1/auth/password/recovery/approve`
- `GET  /api/v1/auth/me`
- `GET  /api/v1/auth/accounts`
- `GET  /api/v1/auth/sessions`
- `DELETE /api/v1/auth/sessions/{id}`
- `POST /api/v1/auth/sessions/revoke-others`

### OAuth2
- `GET  /oauth/authorize`
- `POST /oauth/hosted-login/start`
- `GET  /oauth/hosted-login/{stateId}`
- `POST /oauth/hosted-login/{stateId}/authorize`
- `POST /oauth/hosted-login/{stateId}/consent`
- `POST /oauth/token`
- `POST /oauth/introspect`
- `GET  /oauth/userinfo`
- `POST /oauth/device/authorize`
- `POST /oauth/device/verify`
- `POST /oauth/device/approve`
- `POST /oauth/device/deny`

### Decision et step-up
- `POST /api/v1/auth/step-up`
- `GET  /api/v1/auth/mfa/factors`
- `POST /api/v1/auth/mfa/totp/setup`
- `POST /api/v1/auth/mfa/totp/confirm`
- `POST /api/v1/auth/mfa/webauthn/start`
- `POST /api/v1/auth/mfa/webauthn/register/start`
- `POST /api/v1/auth/mfa/webauthn/register/finish`
- `POST /api/v1/auth/mfa/recovery-codes`
- `DELETE /api/v1/auth/mfa/factors/{id}`
- `POST /api/v1/authz/decision`

`authz/decision` est une prévisualisation de permission liée à une session utilisateur active.
L'autorisation finale reste dans le service propriétaire de la ressource. Les comptes de service
ne passent pas par cet endpoint. Une session navigateur est authentifiée par son cookie HTTP-only
et ses permissions de membership; un access token utilisateur doit en plus porter le scope
`drive:read`, `drive:write` ou `drive:admin` adapté à l'action, ainsi que respecter son éventuel
binding DPoP ou mTLS.

### gRPC interne

- `nvbes.identity.internal.v1.IdentityInternalService/IntrospectAccessToken`

Le listener gRPC privé utilise `NVBES_IDENTITY_GRPC_PORT` (port `4010` par défaut). Les clients
internes s'authentifient avec leur client OAuth confidentiel dans les metadata gRPC. Les endpoints
OAuth standard `/oauth/token`, `/oauth/introspect` et `/oauth/revoke` restent disponibles en HTTP.

### Workspaces
- `GET  /api/v1/workspaces`
- `POST /api/v1/workspaces`
- `GET  /api/v1/workspaces/{id}`
- `PATCH /api/v1/workspaces/{id}`
- `GET  /api/v1/workspaces/{id}/service-accounts`
- `POST /api/v1/workspaces/{id}/service-accounts`

### Members
- `GET  /api/v1/workspaces/{id}/members`
- `POST /api/v1/workspaces/{id}/invitations`
- `PATCH /api/v1/workspaces/{id}/members/{memberId}`
- `DELETE /api/v1/workspaces/{id}/members/{memberId}`
- `POST /api/v1/invitations/accept`

### Legal et securite
- `GET  /api/v1/legal/consents`
- `POST /api/v1/legal/consent`
- `POST /api/v1/legal/consent/revoke`
- `GET  /api/v1/workspaces/{id}/security-events`
- `GET  /api/v1/workspaces/{id}/security-events/export`
- `GET  /api/v1/workspaces/{id}/recovery-reviews`
- `GET  /api/v1/workspaces/{id}/worker-queue/status`

## Contrats frontend/backend importants

- Le login web canonique est un flux challenge: identifier -> password -> MFA optionnel. Il n'existe pas de route `POST /api/v1/auth/login` exposee par le routeur.
- Les sessions web sont portees par cookie HTTP-only `session` ou `__Host-session`; les clients service peuvent aussi fournir `Authorization: Bearer`.
- `POST /api/v1/auth/mfa/webauthn/register/start` accepte `kind = "passkey" | "security_key"` et le conserve dans `mfa_factors.factor_data.kind`.
- Les surfaces Billing publiques sont portees par `billing-service`; Identity fournit l'introspection et l'autorisation.
- `GET /api/v1/legal/consents` retourne directement un tableau de consentements actifs; la revocation prend `consent_type` et `document_version`.

## Configuration

Variables d'environnement (dans `.env`) :
```
DATABASE_URL=postgresql://user:pass@localhost/nvbes_identity
JWT_SECRET=your-secret-key
NVBES_IDENTITY_GRPC_PORT=4010
NVBES_IDENTITY_GRPC_ENDPOINT=http://127.0.0.1:4010
```

## Migration

```bash
# Créer la base
createdb nvbes_identity

# Lancer les migrations
sqlx migrate run --database-url postgresql://user:pass@localhost/nvbes_identity
```

## Lancement

```bash
cargo run -p nvbes-account-service
```

## Intégration avec les autres produits

Les produits (Drive, etc.) utilisent `packages/identity-sdk` pour :
1. Rediriger vers `/oauth/authorize`
2. Échanger le code contre un `access_token`
3. Sélectionner un contexte workspace si nécessaire
4. Utiliser le `access_token` pour appeler les APIs produits

Pour les flux machine:
1. Créer ou rattacher un `service account` workspace-scopé dans Identity
2. Émettre un client OAuth `service`
3. Obtenir un token via `client_credentials`
4. Introspecter les tokens machine et délégués avant les actions sensibles côté produit

Contrat vendable V1:

- Universal Login hébergé via `/oauth/hosted-login/*` pour les produits browser.
- Authorization code + PKCE `S256` pour les clients publics.
- Discovery OIDC, JWKS, userinfo, introspection et revocation font partie du contrat d'intégration.
- Refresh tokens rotatifs, usage unique, avec révocation de famille en cas de reuse.
- Clients OAuth par produit avec redirect URIs exactes, scopes, secrets hashés, rotation et révocation.

Le modele cible distingue:

- token global minimal pour `/me`, `/workspaces` et le bootstrapping OAuth;
- token ou contexte cible pour toute action produit sur un workspace;
- token machine pour les intégrations Drive/API.

## Federation et SCIM

Le socle federation expose des contrats stables pour:

- `OIDC discovery` et validation de `id_token`;
- `SAML metadata`, `ACS` et validation de subject confirmation;
- `tenant domains` pour le JIT provisioning;
- `linked identities` pour le rattachement explicite des principals;
- `scim-connectors` pour les scenarios de provisioning systeme-a-systeme.

Rappels d'usage:

- `provider_type` doit etre `oidc` ou `saml`;
- les codes d'erreur publics sont documentes dans `docs/api/v1-contracts.md`;
- en non-development, les endpoints federation/SCIM distants sont valides uniquement en HTTPS sur des cibles publiques;
- le provisioning attendu suit le flux `domaine -> provider -> callback/JIT/SCIM -> session + linked identity`.

## État courant

- Le gros du split backend est déjà fait.
- Les fichiers restants à surveiller sont surtout ceux qui dépassent encore 300 lignes.
- Le prochain travail utile est le cleanup documentaire et la réduction des derniers gros modules.

## Documentation

- Architecture : `docs/architecture/identity-product.md`
- Modele de donnees : `docs/domain/data-model.md`
- Contrats API : `docs/api/v1-contracts.md`
- FinOps : `docs/product/finops-billing.md`
- Pricing : `docs/product/pricing.md`
