# Contrat d'intégration Identity ↔ Account

## Vue d'ensemble

Le service Identity agit comme Authorization Server OAuth 2.1 et OpenID
Provider. Le service Account agit comme Resource Server qui accepte uniquement
les access tokens RS256 émis par Identity pour l'audience
`nvbes-account-service`.

## Contrat Token Identity → Account

### Structure du Access Token

Les access tokens émis par Identity contiennent les claims suivants :

```rust
pub struct AccessTokenClaims {
    pub sub: String,           // Principal ID (UUID)
    pub token_type: String,    // "access"
    pub scope: String,         // Scopes séparés par espace
    pub amr: Vec<String>,      // Authentication methods
    pub iss: String,           // Issuer URL
    pub aud: String,           // "nvbes-account-service"
    pub exp: u64,              // Expiration timestamp
    pub iat: u64,              // Issued at timestamp
    pub nbf: u64,              // Not before timestamp
    pub jti: String,           // JWT ID (UUID)
    pub sid: String,           // Session ID (UUID)
}
```

### Validation côté Account

Le service Account valide les tokens avec le `TokenVerifier` :

1. **Signature** : Vérifie la signature RS256 avec la clé publique d'Identity
2. **Header** : Vérifie `alg=RS256`, `kid` correspondant, `typ=at+jwt`
3. **Claims requis** : `exp`, `iat`, `iss`, `aud`, `sub`, `nbf`
4. **Contenu** : Vérifie `token_type=access`, audience exacte, timestamps valides
5. **Extraction** : Crée un `Principal` avec l'ID et les scopes

### Scopes V1

- `account:read` — lecture profil, préférences, notifications, équipes, consents
- `account:write` — mutation profil, préférences, équipes, consents
- `account:export` — demande et téléchargement d'export RGPD Account-local
- `account:close` — fermeture / annulation de fermeture (exige step-up)

Les scopes granulaires historiques (`teams:*`, `account:profile:*`,
`account:legal:*`) ne font pas partie du contrat V1 actif.

### Step-up Authentication

Actions sensibles :

- **Step-up requis** : `account:close` (fermeture)
- **Méthodes acceptées** : `totp`, `webauthn`
- **Vérification** : `amr` contient une de ces méthodes

## Configuration requise

### Account

```env
NVBES_IDENTITY_TOKEN_ISSUER=https://identity.example.com
NVBES_ACCOUNT_TOKEN_AUDIENCE=nvbes-account-service
NVBES_IDENTITY_TOKEN_KEY_ID=key-id
NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM="<PEM-encoded public key>"
```

### Identity

```env
NVBES_IDENTITY_TOKEN_ISSUER=https://identity.example.com
NVBES_IDENTITY_TOKEN_KEY_ID=key-id
NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM="<PEM-encoded private key>"
NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM="<PEM-encoded public key>"
NVBES_IDENTITY_TOKEN_AUDIENCES=nvbes-account-service,nvbes-billing-service
```

## Sécurité

1. HTTPS obligatoire en production
2. Clés RSA 2048+
3. Rotation périodique des clés JWT
4. Access tokens courts (environ 15 minutes)
5. Refresh tokens opaques et rotatifs côté Identity

## Validation

1. Account refuse les tokens sans audience `nvbes-account-service`
2. Account refuse les tokens expirés
3. Account exige step-up pour `account:close`
4. Identity n'émet que vers des audiences enregistrées
