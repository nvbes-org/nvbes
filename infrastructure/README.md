# Infrastructure

Ce dossier contient l'IaC du socle V1, les environnements, les états Terraform
et le contrat FinOps. La [direction produit](../docs/product/nvbes-product-strategy.md)
est la source de vérité : aucune infrastructure Cloud/Drive ou Enterprise n'est
active par défaut.

## Cible V1

- Cloudflare Free pour DNS, TLS, edge et accès interne ;
- Scaleway Serverless Containers avec `min_scale = 0` et maximum explicite ;
- Scaleway Serverless SQL avec capacité minimale nulle par domaine déployé ;
- Email transactionnel derrière le service Email ;
- staging et previews éphémères ;
- région, registry et observabilité mutualisés quand les frontières restent
  intactes ;
- aucun Kubernetes, load balancer permanent, Redis, queue dédiée ou Object
  Storage produit sans besoin et budget démontrés.

Le coût récurrent total, domaines et fournisseurs compris, vise 20 EUR TTC et
ne doit jamais dépasser 30 EUR TTC.

## Structure active

- `finops/` : contrat budgétaire exécutable et procédure opérateur ;
- `environments/email-production/` : environnement Email actif ;
- `environments/trust-risk-production/` : environnement Trust/Risk actif ;
- `bootstrap/production/` : état distant et identités Terraform ;
- `environments/production/` : primitives générales réellement partagées ;
- `local/` : développement local non permanent ;
- `modules/` et `stacks/` : modules réutilisables ou historiques, à évaluer
  avant usage.

Les overlays historiques `development`, `staging` ou les modules produit ne
constituent pas une cible de déploiement permanente.

## Gate FinOps

Le budget et sa procédure vivent dans [`finops/`](finops/README.md).

Avant toute modification de production :

```bash
pnpm check:finops
```

Toute ressource payante doit avoir un propriétaire, un plafond, une estimation
TTC et une justification mesurée. Les factures et estimations fournisseur
vérifiées manuellement restent l'autorité tant qu'une collecte automatique sûre
et gratuite n'existe pas.

## Déploiement

- appliquer les changements progressivement par environnement protégé ;
- utiliser des artefacts immuables et une approval explicite ;
- ne jamais maintenir un runtime éveillé par une probe ou un cron ;
- désactiver les smoke tests à effet externe sauf activation explicite ;
- vérifier restauration, coûts et modes dégradés avant élargissement du trafic ;
- rétroporter tout changement manuel durable dans l'IaC.

## Secrets

Les secrets sont injectés par l'environnement protégé ou le secret store, jamais
commités dans Git ou dans `*.tfvars`. Chaque service utilise une identité et des
credentials limités à ses propres ressources.

Les plans d'infrastructure antérieurs sont conservés comme historique. Ils ne
peuvent pas contourner le contrat FinOps ou élargir le périmètre V1.
