# Services locaux

## Stockage objet S3 avec SeaweedFS

SeaweedFS fournit l’API S3 locale utilisée par Account et Cloud. Il est lancé
avec le profil `infra` et conserve les objets dans le volume Docker
`nvbes_seaweedfs_data`.

```bash
docker compose --profile infra -f infrastructure/local/docker-compose.yml up -d seaweedfs
```

Endpoint S3 : `http://localhost:8333` · access key : `nvbes` · secret key :
`nvbes-dev-secret`.

Dans le devcontainer, l’endpoint utilisé par les services est
`http://host.docker.internal:18333` et SeaweedFS est disponible sur
`localhost:18333` afin que les signed URLs soient accessibles au navigateur.

## Emails avec MailHog

MailHog capture les emails envoyés par Account et Account Worker sans les
transmettre à Internet.

```bash
docker compose --profile infra -f infrastructure/local/docker-compose.yml up -d mailhog
```

- Interface et aperçu HTML : http://localhost:8025
- SMTP local : `localhost:1025`

La configuration de développement correspondante est dans `.env.example` :
`NVBES_EMAIL_PROVIDER=smtp`, port `1025` et STARTTLS désactivé. Depuis un
devcontainer, utilisez `host.docker.internal` comme hôte SMTP ; depuis un
processus lancé directement sur la machine, `localhost` convient.

Les tests unitaires des templates email vérifient le rendu HTML et
l’échappement des valeurs injectées :

```bash
cargo test -p nvbes-account-service email --lib
```

Les tokens et codes ne sont jamais écrits dans les logs. Pour les tests
manuels et les tests navigateur locaux, récupérez le lien ou le code dans
MailHog.
