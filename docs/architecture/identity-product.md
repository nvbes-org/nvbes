# nvbes Identity — architecture de référence historique

## Statut

Identity est un service actif du socle V1 pour authentification, credentials,
sessions, récupération, MFA, step-up et autorisation liée à l'identité. Les
éléments Enterprise de ce document — organisations, fédération, SCIM,
gouvernance avancée et consoles — sont futurs et ne sont pas des exigences V1.
Voir la [direction produit](../product/nvbes-product-strategy.md) et
l'[architecture du socle](technical-architecture.md).

## Objectif

nvbes Identity fournit les primitives d'identité, d'authentification et
d'autorisation réutilisables par les futurs produits nvbes.

Identity est deploye comme `identity-service`, distinct de `account-service`.
Identity est l'Authorization Server OAuth 2.1 et l'OpenID Provider. Account,
Cloud, Billing, Developer, Enterprise et Backoffice sont des Resource Servers
qui n'acceptent que leur propre audience. La frontiere et les responsabilites
de migration sont fixees par
[`ADR 0005`](../adr/0005-separate-identity-from-account.md).

La crate `nvbes-product-identity` porte les primitives métier qui ne doivent
jamais dépendre d'Account, notamment le traitement des secrets de clients OAuth
et le cycle de vie des identités non vérifiées. `nvbes-product-account` reste
limité au profil, aux préférences et à l'orchestration de la confidentialité.

Le systeme cible doit supporter:

- connexion utilisateur globale multi-tenant;
- acces contextualise par `tenant`, `organization` optionnelle et `workspace`;
- OAuth 2.1 / OIDC pour web, mobile, desktop, IoT et services;
- sessions hybrides avec verification locale rapide et controle central pour les actions sensibles;
- MFA, step-up, clients OAuth approuves, federation entrante et SCIM.

## Decisions Acceptees

- `user` est l'identite humaine principale globale.
- `principal` est la forme canonique d'identite pour l'audit, l'ownership et les integrations machine.
- `tenant` est obligatoire techniquement pour toute isolation et gouvernance securite.
- `organization` est optionnelle et sert de couche intermediaire business/admin.
- `workspace` est le contexte produit et le scope operationnel des actions Drive.
- tout `user` nouvellement cree recoit un workspace personnel dedie.
- le workspace personnel est separe des workspaces de groupe et ne doit pas devenir implicitement un espace d'equipe.
- `tenant_membership`, `organization_membership` et `workspace_membership` sont les sources de verite d'appartenance.
- `user_identities` porte les differents mecanismes de connexion d'un meme utilisateur.
- l'email n'est jamais l'identifiant fort du systeme.
- un utilisateur peut appartenir a plusieurs tenants.
- les invites externes, contractors, bots et service accounts existent comme classes de principals distinctes.
- federation entrante, registry d'IdP, registry de domaines et SCIM sont deja exposes dans l'API V1.

## Hierarchie de Reference

```text
User global
  -> Tenant mandatory
      -> Organization optional
          -> Workspace
```

Interpretation:

- `tenant`: isolation securite, gouvernance identity, policies, clients OAuth approuves, audit, domains verifies.
- `organization`: structure admin/business optionnelle partageant les policies du tenant.
- `workspace`: contexte produit concret pour fichiers, membres, billing produit et permissions V1.

Exemples supportes:

- solo: `tenant personnel + organization null + workspace personnel`
- petite equipe: `tenant + organization null + un ou plusieurs workspaces`
- equipe structuree: `tenant + organization + workspaces`
- grande entreprise: `tenant + plusieurs organizations + plusieurs workspaces`

## Principals et Appartenance

### User global

Le `user` represente une personne dans nvbes, independamment de ses employeurs, workspaces ou tenants.

Il peut:

- avoir plusieurs identites de connexion;
- etre actif dans plusieurs tenants;
- etre suspendu dans un tenant sans etre supprime globalement.

A la creation du user, Identity doit provisionner un workspace personnel dedie et une membership owner. Ce workspace personnel porte ses propres quotas, billing, policies et consentements produit. Il ne doit pas etre fusionne avec les workspaces de groupe.

### User identities

`user_identities` permet de rattacher plusieurs mecanismes d'authentification a un meme `user`:

- password;
- OIDC;
- SAML;
- WebAuthn / passkeys;
- futures identites mobiles ou federations externes.

Une identite de connexion n'est pas une source d'autorisation. Elle prouve seulement qui se connecte.

### Principals techniques

Les principals techniques sont exposes comme `service accounts` workspace-scopés.

Ils servent a:

- representer les integrations machine OAuth2 / M2M;
- porter les ownerships techniques `created_by_principal_id`, `owner_principal_id`, `requested_by_principal_id`;
- recevoir des roles et policies de workspace via le meme moteur d'autorisation que les humains.

### Memberships

Le droit d'appartenir a un espace se porte par memberships scopes:

- `tenant_memberships`
- `organization_memberships`
- `workspace_memberships`

Les memberships portent:

- `status`
- `member_type`
- `source`
- contexte de provisioning si necessaire

Le role d'un utilisateur ne doit pas etre porte directement par `users`.

## Architecture Hybride Tokens / Sessions

### Principe

Le systeme cible utilise une architecture hybride:

- `access tokens JWT` courts verifies localement pour le trafic normal;
- `refresh tokens` opaques, rotatifs, stockes cote serveur dans Redis;
- introspection et/ou `authz/decision` obligatoires pour les actions critiques;
- les preuves recentes de `step_up` vivent dans l'etat de session Redis, pas dans une table SQL dediee.

### Access tokens

Les access tokens sont:

- signes asymetriquement;
- scopes par audience;
- de courte duree;
- verifies localement par les APIs via JWKS.

Ils ne doivent pas embarquer toute la logique d'autorisation fine.

### Refresh tokens

Les refresh tokens sont:

- opaques;
- stockes sous forme de hash;
- soumis a rotation stricte;
- relies a une famille de refresh tokens;
- revoques par famille en cas de reuse detecte.

### Sessions et grants

Le systeme distingue:

- `session globale`: authentification principale du user;
- `workspace access context`: contexte d'acces a un workspace pour un client OAuth donne;
- `step_up_grant`: preuve recente d'un niveau d'authentification fort, conservee dans Redis.

## AuthContext de Reference

Le contrat documentaire a figer est:

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

Interpretation:

- `tenant_id` est toujours present sur un contexte d'acces produit.
- `workspace_id` peut etre absent sur les endpoints globaux comme `/me`, `/workspaces` ou pendant le bootstrapping OAuth.
- `acr` et `amr` decrivent le niveau et les methodes d'authentification.
- `principal_id` est la cle canonique d'audit et d'ownership.
- pour les principals machine, `user_id` est absent et `principal_kind = service_account`.

## Tokens Globaux et Tokens Contextualises

Le modele cible distingue:

- un token global minimal pour `/me`, `/workspaces`, bootstrapping OAuth et selection de contexte;
- un token/context par workspace pour toute action produit;
- un token machine M2M pour les integrations techniques, introspecte avant les actions sensibles.

Le token global ne doit pas donner acces par transitivite a un workspace sensible.

Un access token contextualise doit au minimum etre lie a:

- `tenant_id`
- `workspace_id`
- `client_id`
- `session_id`
- niveau d'authentification courant

## Workspace Access Decision

L'acces effectif a un workspace se decide par evaluation centralisee.

Le contrat logique est:

```text
workspace access decision
  = membership
  + client policy
  + auth level
  + risk/device policy
```

Le fait d'etre connecte globalement n'implique jamais l'acces automatique a tous les workspaces.

## Switch de Workspace

Le switch de workspace suit la regle:

- autorise seulement si `membership + client policy + auth level + device/risk policy` satisfont les exigences du tenant et du workspace cible;
- revalidation legere si le niveau d'assurance courant suffit et que le client est deja autorise;
- `step-up` ou `deny` si le workspace cible est plus strict.

Exemples:

- meme tenant, meme client, meme `AAL2`, policies equivalentes: switch leger;
- workspace cible avec WebAuthn obligatoire ou client non approuve: `step-up` ou refus.

## Autorisation et Policies

L'autorisation suit un modele hybride:

- permissions produit V1 principalement scopees au workspace;
- policies de securite portees d'abord par `tenant`;
- `organization` et `workspace` peuvent durcir les policies heritees;
- un niveau inferieur ne peut jamais affaiblir une policy heritee.

L'ordre d'heritage est:

```text
system -> tenant -> organization -> workspace
```

Les actions critiques doivent appeler un point de decision central, pas seulement se contenter de verifier un JWT localement.

## Contrats V1 Exposes

Endpoints centraux exposes:

- `POST /oauth/introspect`
- `POST /api/v1/authz/decision`
- `POST /api/v1/auth/challenge/identifier`
- `POST /api/v1/auth/challenge/pwd`
- `POST /api/v1/auth/challenge/mfa`
- `POST /api/v1/auth/challenge/webauthn/start`
- `POST /api/v1/auth/mfa/totp/confirm`
- `POST /api/v1/auth/mfa/webauthn/register/start`
- `POST /api/v1/auth/mfa/webauthn/register/finish`

Federation et provisioning exposes:

- `GET /api/v1/tenants/{tenantId}/domains`
- `POST /api/v1/tenants/{tenantId}/domains`
- `POST /api/v1/tenants/{tenantId}/domains/{domainId}/verify`
- `DELETE /api/v1/tenants/{tenantId}/domains/{domainId}`
- `GET /api/v1/tenants/{tenantId}/identity-providers`
- `POST /api/v1/tenants/{tenantId}/identity-providers`
- `PATCH /api/v1/tenants/{tenantId}/identity-providers/{providerId}`
- `DELETE /api/v1/tenants/{tenantId}/identity-providers/{providerId}`
- `GET /api/v1/tenants/{tenantId}/identity-providers/{providerId}/oidc/discovery`
- `GET /api/v1/tenants/{tenantId}/identity-providers/{providerId}/saml/metadata`
- `POST /api/v1/tenants/{tenantId}/linked-identities`
- `GET /api/v1/tenants/{tenantId}/linked-identities`
- `DELETE /api/v1/tenants/{tenantId}/linked-identities/{identityId}`
- `POST /api/v1/tenants/{tenantId}/jit-provisioning`
- `GET /api/v1/tenants/{tenantId}/scim-connectors`
- `POST /api/v1/tenants/{tenantId}/scim-connectors`
- `PATCH /api/v1/tenants/{tenantId}/scim-connectors/{connectorId}`
- `DELETE /api/v1/tenants/{tenantId}/scim-connectors/{connectorId}`

## Domaine OAuth / Clients

Les clients OAuth doivent etre gouvernes par scope et policy:

- un client peut etre global, approuve par tenant, approuve par organization ou approuve par workspace;
- un tenant ou workspace peut autoriser, restreindre ou interdire un client;
- les scopes sensibles exigent approbation explicite;
- les actions critiques exigent introspection ou `authz/decision`.

## Donnees de Reference

Le modele conceptuel minimal comprend:

- `users`
- `user_identities`
- `tenants`
- `organizations`
- `workspaces`
- `tenant_memberships`
- `organization_memberships`
- `workspace_memberships`
- `sessions` (Redis)
- `oauth_clients`
- `oauth_client_policies`
- `refresh_token_families` (Redis)
- `refresh_tokens` (Redis)
- `auth_factors` (SQL)
- `auth_factor_totp`
- `auth_factor_webauthn`
- `auth_recovery_codes`
- `auth_challenges` (Redis)
- `devices`
- `role_assignments`
- `permissions`
- `policy_documents`
- `tenant_domains`
- `scim_provisioning_connectors`
- `federated_identity_providers`

## Statut du Premier Increment

Ce premier increment doit figer:

- le vocabulaire d'architecture;
- la hierarchie `user -> tenant -> organization? -> workspace`;
- le modele de memberships;
- le mode hybride JWT/introspection;
- le contrat `AuthContext`;
- le socle federation/SCIM deja expose par l'API.

Ce premier increment ne promet pas encore:

- une suite enterprise complete autour de federation/SCIM;
- attestation device enterprise complete;
- policy engine finalement implemente.
