# Contrat d'intégration Identity ↔ Account

## Vue d'ensemble

Le service Identity agit comme Authorization Server OAuth 2.1 et OpenID Provider. Le service Account agit comme Resource Server qui accepte uniquement les access tokens RS256 émis par Identity pour l'audience "account".

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
    pub aud: String,           // Audience (ex: "account")
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
4. **Contenu** : Vérifie `token_type=access`, audience correcte, timestamps valides
5. **Extraction** : Crée un `Principal` avec l'ID et les scopes

### Scopes définis

- `account:read` - Lecture du profil utilisateur
- `account:write` - Modification du profil utilisateur
- `account:close` - Fermeture du compte (exige step-up)
- `teams:read` - Lecture des équipes
- `teams:write` - Modification des équipes
- `teams:admin` - Administration des équipes

### Step-up Authentication

Certaines actions sensibles exigent une authentification forte :

- **Step-up requis** : `account:close`, `teams:admin`
- **Méthodes acceptées** : `totp`, `webauthn`
- **Vérification** : Le service Account vérifie `amr` contient une méthode forte

## Endpoints Identity utilisés par Account

### Introspection OAuth 2.0

```
POST /oauth/introspect
Content-Type: application/x-www-form-urlencoded

token=<access_token>
```

Réponse :

```json
{
  "active": true,
  "scope": "account:read teams:read",
  "client_id": "account",
  "principal_id": "uuid",
  "exp": 1234567890,
  "iat": 1234567890,
  "sub": "uuid",
  "aud": "account",
  "iss": "https://identity.example.com"
}
```

### Découverte JWKS

```
GET /.well-known/jwks.json
```

Réponse :

```json
{
  "keys": [
    {
      "kid": "key-id",
      "kty": "RSA",
      "use": "sig",
      "alg": "RS256",
      "n": "...",
      "e": "AQAB"
    }
  ]
}
```

## Configuration requise

### Account service (config)

```env
TOKEN_ISSUER=https://identity.example.com
TOKEN_AUDIENCE=account
TOKEN_KEY_ID=key-id
TOKEN_PUBLIC_KEY_PEM="<PEM-encoded public key>"
```

### Identity service (config)

```env
NVBES_IDENTITY_TOKEN_ISSUER=https://identity.example.com
NVBES_IDENTITY_TOKEN_KEY_ID=key-id
NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM="<PEM-encoded private key>"
NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM="<PEM-encoded public key>"
NVBES_IDENTITY_TOKEN_AUDIENCES=account,billing,platform-operations
```

Platform Operations consomme un JWT opérateur distinct (`typ=operator+jwt`) avec
`aud=platform-operations`, `role=platform_owner`, `amr` MFA et `auth_time`.
Identity l'émet via `POST /api/v1/operator/token` uniquement pour les
principals listés dans `NVBES_IDENTITY_PLATFORM_OPERATOR_PRINCIPALS`, après
session MFA step-up active.

## Sécurité

1. **HTTPS obligatoire** : Toutes les communications doivent être en HTTPS en production
2. **Clés RSA 2048+** : Les clés doivent être au minimum 2048 bits
3. **Rotation des clés** : Les clés JWT doivent être périodiquement rotatées
4. **Courte durée** : Les access tokens ont une durée de 15 minutes
5. **Refresh tokens** : Les refresh tokens sont opaques et rotatifs avec détection de reuse

## Migration de l'ancien système

L'ancien système où Account gérait l'authentification est remplacé par :

1. **Identity** : Gère maintenant l'authentification, credentials, sessions
2. **Account** : Gère uniquement le profil, préférences, équipes
3. **Contract** : Communication via tokens JWT validés localement + introspection optionnelle

## Validation

Pour valider l'intégration :

1. Vérifier que Account refuse les tokens sans audience "account"
2. Vérifier que Account refuse les tokens expirés
3. Vérifier que Account exige step-up pour les actions sensibles
4. Vérifier que Identity accepte les refresh tokens valides
5. Vérifier que la rotation de refresh tokens fonctionne
