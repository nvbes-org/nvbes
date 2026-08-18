# Production email stack

Cette racine Terraform possède exclusivement l'infrastructure email de
production: domaine Scaleway TEM, DNS Cloudflare, événements SNS, webhook,
secret Sentry, dashboard et alertes email.

Son état distant est indépendant de la stack générale. La clé
`production/email/terraform.tfstate` et le verrouillage S3 natif sont fixés dans
`backend.tf`.

## Contrat GitHub `production-email`

Variables:

- `BASE_DOMAIN`;
- `CLOUDFLARE_ZONE_ID`;
- `EMAIL_SENTRY_DSN_ROTATION`, objet JSON blue/green, initialement
  `{"active_slot":"blue","blue_version":1,"green_version":0,"retain_previous":true}`;
- `EMAIL_SENTRY_TRACES_SAMPLE_RATE`, initialement `0`;
- `EMAIL_WORKER_RUNTIME_SECRET_VERSION`, entier monotone à incrémenter lors
  d'une rotation d'un secret runtime autre que Sentry;
- `GRAFANA_EMAIL_CONTACT_POINT`;
- `GRAFANA_PROMETHEUS_DATASOURCE_UID`;
- `GRAFANA_URL`;
- `SCALEWAY_TEM_TERMS_ACCEPTED`;
- `SCW_PROJECT_ID`;
- `TERRAFORM_STATE_BUCKET`, créé par `infrastructure/bootstrap/production`.

Secrets:

- `CLOUDFLARE_API_TOKEN`;
- `EMAIL_DATABASE_URL`;
- `EMAIL_DATA_ENCRYPTION_KEY`;
- `EMAIL_OBSERVABILITY_INTERNAL_TOKEN`;
- `EMAIL_PRODUCER_TOKENS`;
- `EMAIL_RECIPIENT_HMAC_KEY`;
- `EMAIL_SCALEWAY_SECRET_KEY`, credential TEM dédié au runtime et distinct des
  credentials Terraform;
- `EMAIL_SENTRY_DSN`;
- `EMAIL_TERRAFORM_STATE_ACCESS_KEY`;
- `EMAIL_TERRAFORM_STATE_SECRET_KEY`;
- `GRAFANA_SERVICE_ACCOUNT_TOKEN`;
- `SCW_ACCESS_KEY`;
- `SCW_SECRET_KEY`.

Le bucket d'état partagé est créé par `infrastructure/bootstrap/production`.
L'identité email ne peut accéder qu'à
`production/email/terraform.tfstate` et son `.tflock`. Ne jamais stocker ses
credentials, le DSN ou les tokens providers dans un fichier `.tfvars`.

Terraform est l'unique chemin de modification du domaine TEM, de ses DNS, du
topic SNS, de son abonnement et du webhook. Toute ressource préexistante créée
via une console, `scw` ou Wrangler doit être importée avant le premier plan.

## Observabilité email

`email-worker` multiplexe HTTP/1.1 et gRPC/h2c sur son unique port Serverless.
Sa route `/metrics` exige le bearer token interne d'Alloy et ne retourne aucune
métrique aux requêtes publiques non authentifiées.

Terraform crée le secret protégé
`/nvbes/production/email-worker/nvbes-prod-email-worker-sentry-dsn`. Fournir sa
valeur uniquement pendant le plan/apply via `TF_VAR_email_sentry_dsn`. La
variable est éphémère et `scaleway_secret_version.data_wo` est write-only: le
DSN ne doit apparaître ni dans le plan ni dans le state.

Terraform possède le registre privé, le namespace Serverless et le conteneur
`email-worker`. Son ID et son endpoint public sont des outputs, jamais des
entrées GitHub. L'endpoint SNS est dérivé de
`scaleway_container.email_worker.public_endpoint`.

Le workflow reçoit le digest d'une image signée publiée par
`container-release.yml`, vérifie sa signature, puis fournit l'image GHCR
complète via `TF_VAR_email_worker_source_image`. Terraform la copie vers son
registre privé Scaleway sous un tag immuable dérivé du digest. Ce digest devient
également `SENTRY_RELEASE`.

Serverless Containers ne sait pas encore référencer directement Secret
Manager. Terraform démarre donc le binaire en mode `deployment-bootstrap`, qui
n'expose que `/health/live` sans charger de secret, puis utilise un adaptateur
`local-exec` limité pour appliquer en une seule mise à jour `scw` la commande
normale et tous les secrets d'environnement. Les variables Terraform
correspondantes sont `sensitive` et `ephemeral`; elles ne sont pas persistées
dans le plan ou le state. Le runner d'apply doit disposer de Docker Buildx et de
`scw`, être authentifié aux registres GHCR et Scaleway, et pouvoir modifier le
conteneur.

### Rotation Sentry sans interruption

Le secret utilise deux slots stables, `blue` et `green`. Une rotation conserve
explicitement N et N-1 dans Secret Manager pendant le rollout:

1. choisir le slot inactif comme `active_slot`;
2. lui attribuer une version strictement supérieure, conserver
   `retain_previous = true`, fournir le nouveau `TF_VAR_email_sentry_dsn`, puis
   appliquer;
3. vérifier `/health/ready` et `nvbes-email-worker error-reporting-smoke`;
4. dans un apply séparé, passer `retain_previous = false` pour supprimer N-1;
5. remettre `retain_previous = true` lors de la rotation suivante.

`data_wo` doit être configuré sur chaque ressource de version, mais seul le
slot dont `data_wo_version` augmente est réécrit. Le slot inactif conserve donc
sa version distante N-1 même si la même variable éphémère est évaluée pendant
le plan/apply.

Exemple pour passer de `blue = 1` à `green = 2`:

```hcl
email_sentry_dsn_rotation = {
  active_slot     = "green"
  blue_version    = 1
  green_version   = 2
  retain_previous = true
}
```

Si l'update du conteneur échoue, l'apply échoue également et N-1 reste
disponible. Le DSN n'est jamais placé dans les arguments persistants de
`scaleway_container`, qui le feraient entrer dans les artifacts Terraform.

## Initialisation

Copier `backend.hcl.example` hors version control, renseigner uniquement le nom
du bucket, puis fournir les credentials Object Storage via l'environnement:

```bash
terraform init -backend-config=backend.hcl
terraform plan -out=email-production.tfplan
terraform apply email-production.tfplan
```

En CI, utiliser `.github/workflows/deploy.yml`, choisir `production` et
`email-worker`, puis fournir le digest `sha256:...` publié dans le résumé du
workflow `container-release`. Le job protégé sélectionne l'environnement
GitHub `production-email`.

## Migration depuis l'ancien état production

Avant le premier plan, vérifier si l'état de
`infrastructure/environments/production` contient déjà des ressources
`scaleway_tem_*`, `scaleway_mnq_sns_*`, `cloudflare_dns_record.transactional_email_*`,
`scaleway_secret.email_worker_sentry_dsn` ou `grafana_*` email.

Si elles existent, les transférer ou les importer dans le nouvel état avant de
planifier l'une des deux stacks. Un déplacement de fichiers HCL ne déplace pas
les ressources entre états Terraform. Le premier plan des deux racines doit
être relu et ne doit proposer ni destruction ni recréation de ces ressources.
