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

## Emails transactionnels

Le runtime global `email-worker` envoie les emails de développement vers
Mailpit. Son interface est disponible sur `http://127.0.0.1:8025` et son port
SMTP local est `127.0.0.1:11025`.

Dans le devcontainer, l’interface est publiée sur
`http://127.0.0.1:18025`; le port SMTP hôte reste `11025`.

```bash
docker compose --profile infra -f infrastructure/local/docker-compose.yml up -d mailpit
```

Les services produits soumettent toujours leurs commandes au worker par gRPC;
gRPC/h2c, les probes et le webhook HTTP partagent le port `3040`. Seul le
worker se connecte à Mailpit. Le provider SMTP est refusé hors des
environnements `development` et `test`.

Le conteneur PostgreSQL crée `nvbes_email` à l’initialisation. Sur un volume
local déjà existant, créez-la une seule fois avec
`docker compose -f infrastructure/local/docker-compose.yml exec postgres createdb -U postgres nvbes_email`.

La configuration locale complète est documentée dans `.env.example`. Les
tests de rendu et d’échéance se lancent avec :

```bash
cargo test -p nvbes-email -p nvbes-email-worker
```

Les tokens, codes, destinataires et corps rendus ne sont jamais écrits dans
les logs.
