# Infrastructure

Ce dossier contient l'infrastructure as code V1, les conventions d'environnements et les modules partages.

## Structure

- `environments/development`: overlay OpenTofu/Terraform pour l'environnement development.
- `environments/staging`: overlay OpenTofu/Terraform pour l'environnement staging.
- `bootstrap/production`: bucket de state, chiffrement et identités Terraform.
- `environments/production`: infrastructure générale et observabilité production.
- `stacks/email/production`: stack produit email et état indépendant.
- `modules/terraform-state-backend`: bucket et accès de state réutilisables.
- `modules/scaleway-v1`: socle Scaleway reutilisable.

## Cible V1 minimale

- Cloudflare pour DNS, proxy TLS, WAF minimal et challenge des routes API publiques.
- Scaleway Instances pour API et workers.
- Scaleway Private Network dedie par environnement.
- Scaleway Managed PostgreSQL avec endpoint prive, chiffrement au repos et backups automatiques.
- Scaleway Object Storage prive avec versioning, CORS limite et lifecycle rules.
- IAM runtime par environnement.
- Inventaire des secrets a provisionner dans Secret Manager sans inscrire les valeurs dans Git.

## FinOps gate

The executable V1 production budget and its operating procedure live in
[`finops/`](finops/README.md). Run `pnpm check:finops` before applying any
production infrastructure change.

## Roadmap production Scaleway

La trajectoire cible est documentee dans
[Plan Scaleway - Fondation production](../docs/blueprint/scaleway-production-foundation.plan.md).

Ordre de priorite:

1. Load Balancer + Public Gateway.
2. Secret Manager + Key Manager.
3. Container Registry.
4. Serverless Jobs.
5. Queues / RabbitMQ.
6. Edge Services.

Chaque produit ajoute doit supprimer une responsabilite operationnelle existante:
exposition reseau directe, gestion de secrets statiques, build artisanal, worker batch permanent, queue fragile ou diffusion statique lente.

## Commandes

```bash
cd infrastructure/environments/development
cp terraform.tfvars.example terraform.tfvars
tofu init
tofu plan
```

Remplacer `development` par `staging` pour valider l'overlay staging. La
production utilise Terraform et un backend distant obligatoire; suivre son
README plutôt que ces commandes locales.

## Secrets

Les overlays creent l'identite runtime et exposent un inventaire des secrets attendus. Les valeurs reelles doivent etre injectees par le secret store ou la CI, pas committees dans `*.tfvars`.
