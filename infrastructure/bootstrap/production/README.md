# Production Terraform state bootstrap

Cette stack crée les seules ressources dont Terraform dépend avant toutes les
autres stacks de production:

- un bucket Object Storage partagé, privé et versionné;
- le chiffrement serveur AES-256;
- une identité et une API key par stack;
- une bucket policy limitée à une clé de state et son `.tflock` par identité,
  sans droit d'énumération du bucket;
- une condition TLS obligatoire sur chaque accès autorisé.

Le bucket utilise des clés indépendantes:

```text
production/bootstrap/terraform.tfstate
production/platform/terraform.tfstate
production/email/terraform.tfstate
production/billing/terraform.tfstate
production/identity/terraform.tfstate
production/trust-risk/terraform.tfstate
```

L'identité state Trust/Risk est pré-bootstrapée séparément afin que son premier
déploiement puisse initialiser son backend. La racine bootstrap conserve la
propriété déclarative de son accès au bucket via
`external_state_application_ids`; elle n'essaie pas de recréer son application
IAM ni ses credentials.

Object Lock/WORM est volontairement désactivé: il ne remplace pas le lockfile
Terraform et pourrait empêcher les mises à jour normales du state.

## Premier bootstrap

Le dossier `initial` contient exactement le même module que la racine finale,
mais utilise le backend local implicite. Exécuter cette phase uniquement depuis
un volume local chiffré et contrôlé:

```bash
cd infrastructure/bootstrap/production/initial
cp terraform.tfvars.example terraform.tfvars
terraform init
terraform plan -out=bootstrap.tfplan
terraform apply bootstrap.tfplan
```

Le state local contient les API keys de toutes les stacks. Ne pas l'envoyer
dans des logs ou artifacts. Distribuer chaque paire uniquement vers
l'environnement GitHub correspondant, puis utiliser les credentials
`bootstrap` comme `AWS_ACCESS_KEY_ID` et `AWS_SECRET_ACCESS_KEY`.

Pour migrer sans changer les adresses de ressources, copier temporairement le
state local dans le dossier parent, initialiser le backend distant avec
`-migrate-state`, puis vérifier le state distant:

```bash
cd ..
cp initial/terraform.tfstate terraform.tfstate
terraform init -migrate-state -backend-config=backend.hcl
terraform state list
```

Après vérification, supprimer toutes les copies locales de state et les plans
depuis le volume chiffré. Ne jamais exécuter `apply` simultanément pendant la
migration.

## GitHub

La variable non secrète commune est `TERRAFORM_STATE_BUCKET`. Chaque
environnement produit conserve ses propres secrets d'accès/secret key. Le
bootstrap doit utiliser un environnement `production-bootstrap` protégé par
reviewers obligatoires et sans bypass administrateur.

Avant le premier apply d'une stack produit, lancer deux opérations concurrentes
contrôlées sur sa clé et confirmer que la seconde échoue sur le `.tflock`.

## Cache CI Scaleway additif

Le module `scaleway-ci-cache` est ajouté à la racine bootstrap existante. Il ne
réutilise ni ne remplace les registries Email/Trust-Risk ou les buckets produit :
il crée un projet `nvbes-ci-cache` isolé, un registry BuildKit, un bucket S3 et
une identité CI propres.

Compléter les variables suivantes dans le `terraform.tfvars` bootstrap :

```hcl
scaleway_organization_id = "<organization UUID>"
ci_cache_bucket_name     = "<globally unique bucket name>"
```

Fournir également `GITHUB_TOKEN` avec les droits d'administration des
Environments et Actions secrets du dépôt. Le token sert uniquement au provider
GitHub et ne doit pas être écrit dans un fichier Terraform.

Pour le premier déploiement, limiter explicitement le plan aux nouvelles
ressources puis appliquer exactement ce plan :

```bash
terraform init -reconfigure -backend-config=backend.hcl
terraform plan -target=module.ci_cache -out=ci-cache-bootstrap.tfplan
terraform apply ci-cache-bootstrap.tfplan
```

Si le projet, le registry, le bucket ou l'environnement GitHub ont déjà été
créés manuellement, les importer avant le plan au lieu de les recréer :

```bash
terraform import module.ci_cache.scaleway_account_project.ci_cache <project-id>
terraform import module.ci_cache.scaleway_registry_namespace.ci_cache fr-par/<namespace-id>
terraform import module.ci_cache.scaleway_object_bucket.ci_cache fr-par/<bucket-name>
terraform import module.ci_cache.github_repository_environment.ci_cache nvbes:production-ci-cache
```

L'environnement `production-bootstrap` doit contenir les secrets
`BOOTSTRAP_TERRAFORM_STATE_ACCESS_KEY`,
`BOOTSTRAP_TERRAFORM_STATE_SECRET_KEY`, `SCW_ACCESS_KEY`, `SCW_SECRET_KEY` et
`CI_CACHE_GITHUB_TOKEN`, ainsi que les variables `SCW_ORGANIZATION_ID`,
`SCW_PROJECT_ID`, `SCW_CI_CACHE_BUCKET_NAME` et `TERRAFORM_STATE_BUCKET`.

Le workflow hebdomadaire alterne deux clés de 60 jours décalées de 30 jours.
GitHub reçoit toujours le slot dont l'échéance est la plus lointaine. Le bucket
supprime automatiquement les objets reproductibles après 30 jours ; le registry
utilise un tag `buildcache` stable par image.
