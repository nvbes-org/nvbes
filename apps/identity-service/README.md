# nvbes Identity service

Runtime actif propriétaire des credentials, sessions, mécanismes de récupération,
MFA, step-up et émission de tokens du socle V1.

## Surface HTTP active

- `POST /api/v1/auth/register` — inscription email/mot de passe, **fermée par
  défaut hors développement** (`NVBES_IDENTITY_PUBLIC_SIGNUP`)
- `POST /api/v1/auth/login` — session navigateur (`nvbes_sid`) + reprise OAuth
  via `return_to`
- `POST /api/v1/auth/logout` — révocation de session et cookie
- `GET /oauth/authorize` — Authorization Code + PKCE ; sans session, 401 avec
  `return_to` ou redirection vers `NVBES_IDENTITY_LOGIN_URL`
- `POST /oauth/token` — `application/x-www-form-urlencoded` (`authorization_code`,
  `refresh_token`)
- `POST /oauth/introspect`, `POST /api/v1/authz/decision`
- `GET /.well-known/openid-configuration`, `GET /.well-known/jwks.json`
- probes `/health/live`, `/health/ready`, `/metrics` (Bearer)

Les inscriptions publiques restent désactivées jusqu'au GO explicite. Le login
et OAuth 2.1 restent disponibles pour les clients internes et les comptes
synthétiques.

## Commandes

```bash
pnpm nx run identity-service:check
pnpm nx run identity-service:test
DATABASE_URL=postgres://localhost/nvbes_identity_test \
  pnpm nx run identity-service:test:database
NVBES_IDENTITY_DATABASE_URL=postgres://localhost/nvbes_identity \
  cargo run -p nvbes-identity-service -- migrate
```

Le smoke exige en plus `NVBES_IDENTITY_SYNTHETIC_EMAIL`,
`NVBES_IDENTITY_SYNTHETIC_PASSWORD` et
`NVBES_IDENTITY_SYNTHETIC_RECOVERED_PASSWORD`. Il ne journalise aucun de ces
secrets ni les tokens éphémères.

`synthetic-mfa-smoke` exige aussi `NVBES_IDENTITY_MFA_ENCRYPTION_KEY`, clé de
32 octets encodée en base64. Hors développement, le runtime refuse de démarrer
sans cette clé. Le smoke ne restitue jamais le secret TOTP.

La rotation utilise `NVBES_IDENTITY_MFA_KEY_VERSION` pour la clé active et la
paire optionnelle `NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY` /
`NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION`. Exécuter `rotate-mfa-key`, vérifier
que le résultat couvre tous les facteurs de l'ancienne version, puis seulement
retirer la clé précédente. La rotation est transactionnelle et auditée.

`synthetic-auth-email-smoke` ajoute la preuve Identity → Email. Il exige
`NVBES_IDENTITY_RECOVERY_BASE_URL` en HTTPS ainsi que la configuration standard
`NVBES_EMAIL_GRPC_ENDPOINT` et `NVBES_EMAIL_GRPC_AUTH_TOKEN`. La récupération
n'est consommée qu'après l'acceptation durable de la commande par Email.

L'image OCI reproductible s'exécute sans privilèges sous l'UID/GID `10001`.
Hors développement, le runtime exige également
`NVBES_IDENTITY_METRICS_TOKEN`, `SENTRY_DSN`, `NVBES_OTLP_ENDPOINT` et
`NVBES_OTLP_AUTHORIZATION_HEADER`. `/metrics` refuse toute requête sans bearer
token exact ; `/health/ready` refuse le trafic lorsque PostgreSQL est
indisponible, tandis que `/health/live` reste superficielle.
