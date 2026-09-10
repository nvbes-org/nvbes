# nvbes Identity service

Runtime actif propriétaire des credentials, sessions, mécanismes de récupération,
MFA, step-up et émission de tokens du socle V1.

Le premier incrément fournit uniquement le socle opérationnel fermé :

- configuration stricte hors développement ;
- base PostgreSQL indépendante et migrations explicites ;
- probes HTTP peu coûteuses ;
- liveness superficielle, readiness dépendante de PostgreSQL et métriques
  Prometheus protégées ;
- traces OTLP authentifiées et remontée Sentry obligatoires en production ;
- tables minimales pour principals, identifiants, credentials, audit et outbox.
- smoke interne couvrant création synthétique, authentification, rotation de
  session et récupération de mot de passe à usage unique.
- TOTP chiffré par AES-256-GCM, compteur anti-rejeu et grant de step-up borné.

Aucune route d'inscription ou d'authentification publique n'est exposée par cet
incrément. `/auth/register` reste absent. Les comptes humains ne sont créés que
par invitation opérateur (`identity_invitations`, code stocké en hash SHA-256).
Les inscriptions publiques restent fermées jusqu'au GO explicite.

## Contrat de token JWT RS256

Contrat canonique : `contracts/identity/access-token.v1.schema.json` et
`contracts/identity/scopes.v1.json`.

| Audience                | Scopes autorisés                                                   |
| ----------------------- | ------------------------------------------------------------------ |
| `nvbes-account-service` | `account:read`, `account:write`, `account:export`, `account:close` |
| `nvbes-billing-service` | `billing:read`, `billing:checkout`                                 |

- Algorithme : RS256, en-tête `typ=at+jwt`, claim `token_type=access`.
- Durée de vie : 900 secondes (15 minutes).
- `amr` : `pwd` obligatoire ; `totp` ou `webauthn` ajouté lorsque la session
  porte un grant step-up actif (`step_up_method`, TTL 10 minutes).
- Révocation : l'introspection session (`identity_sessions.revoked_at`) rend le
  token inactif immédiatement. Sans introspection, les consommateurs doivent
  respecter `exp` ; la fenêtre résiduelle maximale est donc **15 minutes**.
- Preuve synthétique : `synthetic-token-smoke` (audit
  `identity.token.synthetic_proven`, sans persister le JWT).

## Commandes

L'activation du registre `NVBES_IDENTITY_OAUTH_CLIENTS_JSON` exige
`NVBES_IDENTITY_RATE_LIMIT_KEY`, clé aléatoire de 32 octets encodée en base64,
stable et partagée entre les répliques. Les quotas HTTP utilisent l'adresse de
la connexion TCP ; les en-têtes proxy ne sont pas des preuves de source.
Voir [les quotas et leur configuration](../../docs/architecture/identity-token-lifecycle.md#quotas-des-routes-http)
avant d'activer les routes derrière un proxy. Les lots A à D restent en cours.

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

`synthetic-invitation-smoke` exige `NVBES_IDENTITY_SYNTHETIC_INVITER_EMAIL`,
`NVBES_IDENTITY_SYNTHETIC_INVITED_EMAIL` et `NVBES_IDENTITY_SYNTHETIC_PASSWORD`.
Il prouve qu'un code d'invitation n'est jamais stocké en clair et qu'une
acceptation est à usage unique.

`synthetic-token-smoke` exige en plus `NVBES_IDENTITY_TOKEN_ISSUER`,
`NVBES_IDENTITY_TOKEN_KEY_ID`, `NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM`,
`NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM`, `NVBES_IDENTITY_TOKEN_AUDIENCES` et
`NVBES_IDENTITY_SYNTHETIC_TOKEN_AUDIENCE` (`nvbes-account-service` en production).

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
