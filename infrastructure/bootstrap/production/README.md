# Production Terraform state bootstrap

Cette stack crée les seules ressources dont Terraform dépend avant toutes les
autres stacks de production:

- un bucket Object Storage partagé, privé et versionné;
- le chiffrement serveur AES-256;
- une identité et une API key par stack;
- une bucket policy limitée à une clé de state et son `.tflock` par identité;
- une condition TLS obligatoire sur chaque accès autorisé.

Le bucket utilise des clés indépendantes:

```text
production/bootstrap/terraform.tfstate
production/platform/terraform.tfstate
production/email/terraform.tfstate
production/billing/terraform.tfstate
production/identity/terraform.tfstate
```

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
