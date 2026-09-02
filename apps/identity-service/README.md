# nvbes Identity service

Runtime actif propriétaire des credentials, sessions, mécanismes de récupération,
MFA, step-up et émission de tokens du socle V1.

Le premier incrément fournit uniquement le socle opérationnel fermé :

- configuration stricte hors développement ;
- base PostgreSQL indépendante et migrations explicites ;
- probes HTTP peu coûteuses ;
- tables minimales pour principals, identifiants, credentials, audit et outbox.
- smoke interne couvrant création synthétique, authentification, rotation de
  session et récupération de mot de passe à usage unique.
- TOTP chiffré par AES-256-GCM, compteur anti-rejeu et grant de step-up borné.

Aucune route d'inscription ou d'authentification publique n'est exposée par cet
incrément. Elles seront ajoutées par parcours verticaux complets. Cela maintient
les inscriptions publiques fermées jusqu'au GO explicite.

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
