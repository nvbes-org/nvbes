# Modele de Donnees de Reference

## Principe Central

Le modele de reference ne prend plus `workspace` comme unite unique d'isolation identity.

Le systeme separe:

- isolation securite et gouvernance: `tenant`
- structure business optionnelle: `organization`
- espace produit: `workspace`

Tout principal humain ou machine appartient toujours a un `tenant`, meme dans les cas solo ou petite equipe.

## Entites Cibles

### User

Identite principale globale.

- `id`
- `primary_email`
- `display_name`
- `global_status`
- `created_at`
- `updated_at`

Regles:

- l'email n'est pas un identifiant fort;
- le `user` est global et peut exister dans plusieurs tenants.

### UserIdentity

Mecanismes de connexion rattaches a un `user`.

- `id`
- `user_id`
- `provider_type`
- `provider_id`
- `subject`
- `email`
- `email_verified`
- `tenant_id` nullable
- `created_at`

Exemples de `provider_type`:

- `password`
- `oidc`
- `saml`
- `webauthn`
- `passkey`

### Tenant

Unite obligatoire d'isolation securite et de gouvernance identity.

- `id`
- `kind`
- `name`
- `slug`
- `security_tier`
- `status`
- `created_at`

Exemples de `kind`:

- `personal`
- `team`
- `enterprise`
- `system`

### Organization

Structure intermediaire optionnelle entre `tenant` et `workspace`.

- `id`
- `tenant_id`
- `name`
- `slug`
- `parent_organization_id` nullable
- `status`
- `created_at`

### Workspace

Contexte produit concret.

- `id`
- `tenant_id`
- `organization_id` nullable
- `workspace_type`
- `name`
- `security_profile_id` nullable
- `plan_id`
- `trial_started_at`
- `trial_ends_at`
- `created_at`
- `updated_at`

Regle:

- un `workspace` appartient toujours a un `tenant` et peut avoir `organization_id = NULL`.

### TenantMembership

- `tenant_id`
- `user_id`
- `member_type`
- `status`
- `source`
- `created_at`
- `updated_at`

Exemples de `member_type`:

- `owner`
- `admin`
- `member`
- `guest`
- `contractor`

### OrganizationMembership

- `organization_id`
- `user_id`
- `status`
- `source`
- `created_at`
- `updated_at`

### WorkspaceMembership

- `workspace_id`
- `user_id`
- `role`
- `status`
- `source`
- `created_at`
- `updated_at`

Les permissions produit V1 restent principalement portees par `workspace_memberships`.

### Session

Session globale d'authentification, stockee dans Redis.

- `id`
- `user_id`
- `tenant_id`
- `client_id`
- `device_id` nullable
- `created_at`
- `last_seen_at`
- `expires_at`
- `revoked_at`
- `ip`
- `user_agent`

### Device

Representation d'un device connu ou atteste.

- `id`
- `user_id`
- `tenant_id`
- `device_type`
- `trust_level`
- `platform`
- `attested_at` nullable
- `revoked_at` nullable
- `created_at`

### OAuthClient

Client OAuth/OIDC gouverne par scope et policy.

- `id`
- `client_id`
- `client_secret_hash` nullable
- `owner_scope_type`
- `owner_scope_id`
- `client_type`
- `name`
- `redirect_uris`
- `allowed_grant_types`
- `status`
- `created_at`

### OAuthClientPolicy

Approbation ou restriction d'usage d'un client a un scope donne.

- `id`
- `client_id`
- `scope_type`
- `scope_id`
- `allowed_scopes`
- `required_acr`
- `device_policy`
- `status`
- `created_at`

### RefreshTokenFamily

Famille de refresh tokens stockee dans Redis.

- `id`
- `session_id`
- `user_id`
- `tenant_id`
- `client_id`
- `workspace_id` nullable
- `revoked_at`
- `created_at`

### RefreshToken

Refresh token opaque stocke dans Redis.

- `id`
- `family_id`
- `token_hash`
- `rotated_from_id` nullable
- `replaced_by_id` nullable
- `reuse_detected_at` nullable
- `expires_at`
- `revoked_at`
- `created_at`
- `last_used_at`

### AuthFactor

Facteur MFA generique, source de verite relationnelle conservee en SQL.

- `id`
- `user_id`
- `tenant_id`
- `factor_type`
- `status`
- `label`
- `created_at`
- `confirmed_at`
- `revoked_at`
- `last_used_at`

### AuthFactorTotp

- `factor_id`
- `secret_encrypted`
- `digits`
- `period_seconds`
- `algorithm`
- `last_used_counter`
- `issuer`

### AuthFactorWebauthn

- `factor_id`
- `credential_id_hash`
- `credential_id_encrypted` nullable
- `public_key`
- `sign_count`
- `transports`
- `aaguid`
- `rp_id`
- `user_verification_required`
- `backup_eligible`
- `backup_state`

### AuthRecoveryCode

- `id`
- `user_id`
- `factor_id` nullable
- `code_hash`
- `created_at`
- `consumed_at`

### AuthChallenge

Demande ponctuelle d'authentification ou de step-up, stockee dans Redis.

- `id`
- `user_id`
- `session_id`
- `tenant_id`
- `workspace_id` nullable
- `purpose`
- `required_level`
- `allowed_factor_types`
- `factor_id` nullable
- `expires_at`
- `consumed_at`
- `failed_attempts`
- `metadata`
- `created_at`

### AuthStepUpGrant

Preuve recente centralisee d'un niveau d'authentification, stockee dans Redis.

- `id`
- `user_id`
- `session_id`
- `tenant_id`
- `workspace_id` nullable
- `challenge_id`
- `factor_id`
- `level`
- `issued_at`
- `expires_at`
- `revoked_at`

### Permission

- `id`
- `code`
- `description`
- `created_at`

### RoleAssignment

Contrat documentaire a figer:

- `principal_id`
- `role_id`
- `scope_type`
- `scope_id`
- `created_at`

`scope_type`:

- `tenant`
- `organization`
- `workspace`

### Workspace Ownership

Les workspaces portent un ownership double-ecrit pendant la transition:

- `owner_user_id` pour la compatibilite legacy humaine;
- `owner_principal_id` comme ownership canonique.

Les integrations machine doivent toujours lire et ecrire `owner_principal_id`.

### Privacy Request

Les demandes de privacy distinguent le sujet et l'acteur:

- `subject_user_id` pour le sujet humain concerne;
- `requested_by_principal_id` pour l'acteur canonique de la demande.

Le champ `requested_by` reste present pour compatibilite, mais ne doit plus etre considere comme l'identite canonique de l'acteur.

### PolicyDocument

Support documentaire pour policies heritables.

- `id`
- `scope_type`
- `scope_id`
- `policy_type`
- `document`
- `version`
- `created_at`

### Federation / SCIM Scaffold

Non implementes maintenant, mais reserves:

#### TenantDomain

- `id`
- `tenant_id`
- `domain`
- `verified_at` nullable
- `created_at`

#### ScimProvisioningConnector

- `id`
- `tenant_id`
- `provider`
- `status`
- `base_url`
- `created_at`

#### FederatedIdentityProvider

- `id`
- `tenant_id`
- `provider_type`
- `name`
- `issuer`
- `metadata_url`
- `status`
- `created_at`

## AuthContext de Reference

Le contrat logique a utiliser dans les docs et les futures interfaces est:

```text
AuthContext
  user_id
  session_id
  tenant_id
  workspace_id optional
  client_id
  acr
  amr
  device_id optional
```

## Regles Metier

- un `tenant` existe toujours, meme pour un usage solo;
- un `organization` est optionnelle;
- un `workspace` appartient toujours a un `tenant`;
- un `workspace` peut exister sans `organization`;
- un utilisateur peut appartenir a plusieurs tenants;
- les roles ne sont jamais portes directement par `users`;
- l'autorisation produit V1 reste principalement au niveau workspace;
- les policies de securite s'heritent `system -> tenant -> organization -> workspace`;
- un niveau inferieur peut durcir une policy, jamais l'affaiblir;
- `workspace_id` n'est plus la seule cle d'isolation documentaire;
- les comptes service et bots ne reutilisent pas les memes parcours que les users humains;
- les actions critiques doivent pouvoir exiger introspection et/ou decision centrale;
- les tokens globaux minimaux ne donnent pas automatiquement acces aux workspaces sensibles.

## Workspace Access Decision

L'acces effectif a un workspace repose sur:

```text
membership
+ client policy
+ auth level
+ risk/device policy
```

Le switch de workspace doit reevaluer ces dimensions avant emission d'un contexte ou token cible.
