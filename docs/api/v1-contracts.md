# Contrats API V1

## Principes

- Le systeme identity de reference suit `principal global -> tenant -> organization optionnelle -> workspace`.
- Les routes produit sont scopees par `workspaceId`.
- Un token global minimal peut servir `/me`, `/workspaces` et le bootstrapping OAuth; les actions produit utilisent un contexte workspace.
- Le backend verifie les permissions a chaque requete.
- Les actions sensibles peuvent exiger introspection et/ou `authz/decision`.
- Les routes publiques de partage n'exposent jamais les object keys.
- Les signed URLs sont courtes.
- Les tokens de partage sont stockes sous forme de hash.
- L'API publique versionnee est documentee dans [API Publique V1](public-api-v1.md).

Contrats documentaires de reference:

- `AuthContext`: `principal_id`, `principal_kind`, `user_id`, `session_id`, `tenant_id`, `workspace_id optional`, `client_id`, `acr`, `amr`, `device_id optional`
- `workspace access decision`: `membership + client policy + auth level + risk/device policy`
- endpoints:
  - `POST /oauth/introspect`
  - `POST /api/v1/authz/decision`
  - `POST /api/v1/auth/challenge/identifier`
  - `POST /api/v1/auth/challenge/pwd`
  - `POST /api/v1/auth/challenge/mfa`
  - `POST /api/v1/auth/challenge/webauthn/start`
- API keys legacy: lecture et revocation uniquement; la creation est desactivee et la voie canonique pour les integrations machine est `service_accounts` + OAuth clients.

Contrats federation et provisioning V1:

- `GET  /api/v1/tenants/{tenantId}/domains`
- `POST /api/v1/tenants/{tenantId}/domains`
- `POST /api/v1/tenants/{tenantId}/domains/{domainId}/verify`
- `DELETE /api/v1/tenants/{tenantId}/domains/{domainId}`
- `GET  /api/v1/tenants/{tenantId}/identity-providers`
- `POST /api/v1/tenants/{tenantId}/identity-providers`
- `PATCH /api/v1/tenants/{tenantId}/identity-providers/{providerId}`
- `DELETE /api/v1/tenants/{tenantId}/identity-providers/{providerId}`
- `GET  /api/v1/tenants/{tenantId}/identity-providers/{providerId}/oidc/discovery`
- `GET  /api/v1/tenants/{tenantId}/identity-providers/{providerId}/saml/metadata`
- `GET  /api/v1/tenants/{tenantId}/linked-identities`
- `POST /api/v1/tenants/{tenantId}/linked-identities`
- `DELETE /api/v1/tenants/{tenantId}/linked-identities/{identityId}`
- `POST /api/v1/tenants/{tenantId}/jit-provisioning`
- `GET  /api/v1/tenants/{tenantId}/scim-connectors`
- `POST /api/v1/tenants/{tenantId}/scim-connectors`
- `PATCH /api/v1/tenants/{tenantId}/scim-connectors/{connectorId}`
- `DELETE /api/v1/tenants/{tenantId}/scim-connectors/{connectorId}`

### Federation / SCIM readiness

- Le bloc ci-dessous documente le scaffold V1 et les contrats d'API disponibles, pas une suite enterprise finalisée.
- Les payloads V1 sont en `snake_case` et doivent etre valides contractuellement avant persistence.
- `provider_type` accepte `oidc` ou `saml` pour les identity providers federes.
- `provider` et `base_url` des connecteurs SCIM sont normalises avant sauvegarde; `base_url` doit etre une URL absolue.
- En environnement non development, les endpoints federation/SCIM distants doivent etre joignables en HTTPS sur une cible publique.
- Les erreurs publiques de federation sont stabilisees via des codes explicites, notamment:
  - `provider_type_mismatch`
  - `missing_email`
  - `provider_inactive`
  - `domain_not_verified`
  - `invalid_provider_id`
  - `provider_not_found`
  - `connector_not_found`
  - `domain_not_found`
  - `invalid_id_token`
  - `invalid_saml_response`

Scenarios de provisioning supportes:

1. Verifier un domaine tenant, puis creer un identity provider federated, puis lier les identites au moment du callback ou du JIT provisioning.
2. Declarer un connecteur SCIM, verifier la configuration, puis provisionner les comptes depuis le systeme source vers le tenant nvbes.
3. Utiliser les callbacks OIDC/SAML pour creer ou reutiliser un principal, ecrire la session, puis conserver la trace via linked identities et memberships.

## Auth

```http
POST /api/v1/auth/register
POST /api/v1/auth/verify-email
POST /api/v1/auth/verify-email/resend
POST /api/v1/auth/verify-email/change
GET  /api/v1/auth/region
GET  /api/v1/auth/regions
POST /api/v1/auth/challenge/identifier
POST /api/v1/auth/challenge/pwd
POST /api/v1/auth/challenge/mfa
POST /api/v1/auth/challenge/webauthn/start
POST /api/v1/auth/logout
POST /api/v1/auth/password/forgot
POST /api/v1/auth/password/reset
POST /api/v1/auth/password/recovery/approve
GET  /api/v1/auth/accounts
GET  /api/v1/auth/sessions
POST /api/v1/auth/sessions/revoke-others
DELETE /api/v1/auth/sessions/:sessionId
GET  /api/v1/auth/me
POST /api/v1/auth/step-up
GET  /api/v1/auth/mfa/factors
POST /api/v1/auth/mfa/totp/setup
POST /api/v1/auth/mfa/totp/confirm
POST /api/v1/auth/mfa/webauthn/start
POST /api/v1/auth/mfa/webauthn/register/start
POST /api/v1/auth/mfa/webauthn/register/finish
POST /api/v1/auth/mfa/recovery-codes
DELETE /api/v1/auth/mfa/factors/:factorId
```

Contraintes:

- Email verifie avant usage complet du workspace.
- Login et reset password proteges par rate limiting.
- Le login web canonique est le flux challenge `identifier -> pwd -> mfa?`; il n'y a pas de contrat V1 `POST /api/v1/auth/login`.
- Les passkeys et cles de securite utilisent le meme facteur `webauthn`, distingue par `kind = "passkey" | "security_key"` dans `factor_data`.
- Tokens de verification et reset password courts, usage unique et expires.
- Sessions expirees et renouvelables selon une politique documentee.
- Changement de mot de passe invalide les sessions existantes.
- Changement de role sensible invalide les sessions concernees.
- MFA interne obligatoire pour les comptes administrateurs nvbes.
- Le MFA client est documente comme fondation enterprise tier; l'exposition produit peut rester progressive.

## OAuth et Device Flow

```http
GET  /oauth/authorize
POST /oauth/token
POST /oauth/introspect
GET  /oauth/userinfo
POST /oauth/device/authorize
POST /oauth/device/verify
POST /oauth/device/approve
POST /oauth/device/deny
```

Contraintes:

- Les routes OAuth ne sont pas sous `/api/v1`.
- Universal Login expose `POST /oauth/hosted-login/start`, `GET /oauth/hosted-login/:stateId`, `POST /oauth/hosted-login/:stateId/authorize` et `POST /oauth/hosted-login/:stateId/consent`.
- Browser products use authorization code + PKCE through Universal Login.
- Public clients require `S256` PKCE.
- Implicit and password grants are not product contracts.
- Refresh tokens are one-time-use; reuse revokes the refresh-token family.
- OIDC discovery, OAuth authorization server metadata, JWKS, userinfo, introspection and revocation are product integration contracts.
- Hosted-login state is short-lived, server-side, and never exposes raw PAR payloads or secrets to the browser.
- `device/verify` est public et retourne les informations d'application a afficher.
- `device/approve` et `device/deny` exigent une session utilisateur valide.
- `device/approve` prend `user_code`, `workspace_id`, `organization_id?`, `consent_action?`.

## Workspaces

```http
GET   /api/v1/workspaces
POST  /api/v1/workspaces
GET   /api/v1/workspaces/:workspaceId
PATCH /api/v1/workspaces/:workspaceId
GET   /api/v1/workspaces/:workspaceId/service-accounts
POST  /api/v1/workspaces/:workspaceId/service-accounts
GET   /api/v1/workspaces/:workspaceId/service-accounts/:serviceAccountId
PATCH /api/v1/workspaces/:workspaceId/service-accounts/:serviceAccountId
POST  /api/v1/workspaces/:workspaceId/service-accounts/:serviceAccountId/suspend
POST  /api/v1/workspaces/:workspaceId/service-accounts/:serviceAccountId/reactivate
POST  /api/v1/workspaces/:workspaceId/service-accounts/:serviceAccountId/oauth-clients
POST  /api/v1/workspaces/:workspaceId/service-accounts/:serviceAccountId/oauth-clients:attach
POST  /api/v1/workspaces/:workspaceId/service-accounts/:serviceAccountId/oauth-clients/:clientId/rotate-secret
DELETE /api/v1/workspaces/:workspaceId/service-accounts/:serviceAccountId/oauth-clients/:clientId
```

Contraintes:

- la liste des workspaces peut etre servie depuis un token global minimal;
- toute action sur un workspace suppose un contexte ou token cible;
- le switch de workspace peut demander revalidation legere, `step-up` ou refus selon policy.
- `owner_principal_id` est l'ownership canonique expose par le schema.

## Membres

```http
GET    /api/v1/workspaces/:workspaceId/members
POST   /api/v1/workspaces/:workspaceId/members
PATCH  /api/v1/workspaces/:workspaceId/members/:memberId
DELETE /api/v1/workspaces/:workspaceId/members/:memberId
POST   /api/v1/invitations/accept
```

## Fichiers et Dossiers

Les routes Drive internes sont exposees a la racine du Drive API. Les routes Identity sont exposees sous `/api/v1`, sauf OAuth sous `/oauth`.

```http
GET    /workspaces/:workspaceId/objects?parentId=
POST   /workspaces/:workspaceId/folders
PATCH  /workspaces/:workspaceId/objects/:objectId
POST   /workspaces/:workspaceId/objects/:objectId/move
POST   /workspaces/:workspaceId/objects/:objectId/trash
POST   /workspaces/:workspaceId/objects/:objectId/restore
DELETE /workspaces/:workspaceId/objects/:objectId
```

## Uploads

```http
POST /workspaces/:workspaceId/uploads
POST /workspaces/:workspaceId/uploads/:uploadId/complete
POST /workspaces/:workspaceId/uploads/:uploadId/cancel
```

La creation d'upload retourne une signed upload URL et cree un storage object en etat pending.

La completion valide taille et checksum, puis active le storage object.

## Downloads

```http
POST /workspaces/:workspaceId/objects/:objectId/download-url
```

Retourne une signed download URL courte duree apres verification des permissions.

## Liens de Partage

```http
GET    /workspaces/:workspaceId/share-links
POST   /workspaces/:workspaceId/objects/:objectId/share-links
PATCH  /workspaces/:workspaceId/share-links/:shareLinkId
DELETE /workspaces/:workspaceId/share-links/:shareLinkId
GET    /public/shares/:token
POST   /public/shares/:token/download-url
```

Contraintes:

- Aucun lien public sans expiration en V1.
- TTL par defaut: 7 jours.
- TTL maximum defini par plan.
- Tokens stockes uniquement sous forme de hash.
- Routes publiques soumises a rate limiting dedie.
- Acces publics journalises.
- Revocation effective immediatement.
- Les liens publics V1 servent au telechargement; la preview riche n'est pas dans le scope V1.

## Corbeille

```http
GET /workspaces/:workspaceId/trash
```

## Quota

```http
GET /workspaces/:workspaceId/quota
```

Exemple de reponse:

```json
{
  "usedStorageBytes": 123456,
  "includedStorageBytes": 2147483648,
  "additionalStorageBytes": 0,
  "fileCount": 42,
  "bandwidthOutBytesMonth": 987654
}
```

## Billing

```http
GET  /api/v1/workspaces/:workspaceId/billing/overview
POST /api/v1/workspaces/:workspaceId/billing/checkout
POST /api/v1/workspaces/:workspaceId/billing/portal
GET  /api/v1/workspaces/:workspaceId/billing/usage
GET  /api/v1/workspaces/:workspaceId/billing/entitlements
POST /api/v1/webhooks/stripe
```

Provider V1:

- Stripe est le provider billing cible.
- `checkout` cree une Stripe Checkout Session.
- `portal` cree une Stripe Customer Portal Session.
- Les subscriptions, invoices, taxes et paiements sont geres par Stripe.
- nvbes garde un ledger interne pour usages, quotas, droits produit et estimation de facture.

Objets Stripe a mapper:

- Product.
- Price.
- Customer.
- Subscription.
- SubscriptionItem.
- Checkout Session.
- Customer Portal.
- Invoice.
- Tax.
- Meter ou usage event selon le modele Stripe retenu.
- Webhook Event.

Contraintes webhook:

- Signature du provider billing verifiee avant traitement.
- Rejet des evenements trop anciens ou rejoues.
- Traitement idempotent via identifiant d'evenement provider.
- Journalisation de chaque evenement recu, accepte, rejete ou echoue.
- Traitement asynchrone recommande apres validation de signature.
- Les changements de plan, quota ou statut subscription doivent etre audites.

Contrat checkout/portal:

- `checkout` attend `plan_code`, `success_url?`, `cancel_url?`.
- `portal` attend un objet JSON avec `return_url?`; envoyer `{}` quand aucune URL n'est fournie.
- Les deux repondent `{ "url": "https://..." }`.

Contraintes usage:

- Les usages facturables sont calcules depuis le ledger interne.
- Les snapshots d'usage sont conserves pour audit.
- Les usages envoyes a Stripe doivent etre idempotents.
- Les estimations de facture doivent etre consultables avant facturation.

## Audit

```http
GET /api/v1/workspaces/:workspaceId/security-events
GET /api/v1/workspaces/:workspaceId/security-events/export
GET /api/v1/workspaces/:workspaceId/recovery-reviews
GET /api/v1/workspaces/:workspaceId/worker-queue/status
```

Evenements minimum:

- `auth.login_succeeded`
- `auth.login_failed`
- `auth.password_reset_requested`
- `auth.password_changed`
- `permission.denied`
- `member.invited`
- `member.removed`
- `member.role_changed`
- `file.uploaded`
- `file.downloaded`
- `file.trashed`
- `file.restored`
- `file.deleted_permanently`
- `share_link.created`
- `share_link.accessed`
- `share_link.revoked`
- `billing.updated`
- `workspace.export_requested`
- `workspace.delete_requested`

## Privacy

```http
POST /privacy/export
POST /privacy/delete-account
GET  /privacy/requests/:requestId
POST /workspaces/:workspaceId/privacy/export
POST /workspaces/:workspaceId/privacy/delete
```

## API Publique

La V1 expose une API publique limitee aux integrations fichiers.

Voir [API Publique V1](public-api-v1.md) pour:

- Endpoints publics `/v1`.
- API keys legacy (list/revocation uniquement).
- Scopes.
- Rate limits.
- Erreurs.
- Idempotence.
- OpenAPI.
- Guides de lancement public.
