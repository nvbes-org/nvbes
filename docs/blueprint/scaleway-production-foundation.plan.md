# Plan Scaleway - Fondation production

## Statut

**Plan d'infrastructure historique partiellement remplacé.** Les VM, load
balancers, queues et ressources permanentes décrites ici ne constituent plus la
cible V1. Le socle actif utilise le serverless borné, le scale-to-zero et le
budget global 20/30 EUR TTC. Réutiliser une étape uniquement après une décision
FinOps mesurée. Voir la [direction produit](../product/nvbes-product-strategy.md).

## Objectif

Passer d'une infrastructure "VM + services applicatifs" a une base production plus robuste, sans introduire Kubernetes ni generaliser le serverless trop tot.

La regle de decision est simple: chaque produit Scaleway ajoute doit supprimer une responsabilite operationnelle actuelle. Si un produit ne retire pas une douleur concrete, il reste hors scope.

## Etat actuel

Le module `infrastructure/modules/scaleway-v1` fournit deja une base V1:

- Private Network par environnement.
- Instances Scaleway pour API et worker.
- PostgreSQL manage avec endpoint prive, chiffrement au repos et backups.
- Object Storage prive avec versioning, CORS et lifecycle rules.
- IAM runtime et inventaire des secrets.

Les limites principales a corriger avant production sont:

- Les instances API portent encore une exposition publique directe.
- Il n'y a pas encore de Load Balancer applicatif devant `account-service` et `cloud-service`.
- Les sorties internet des instances privees ne sont pas encore centralisees via Public Gateway.
- Les secrets sont inventories, mais leur provisioning et leur rotation ne sont pas encore industrialises.
- Les artefacts applicatifs ne sont pas encore normalises autour d'images versionnees.
- Les batchs sont encore candidats a rester dans des workers permanents.

## Ordre recommande

1. Load Balancer + Public Gateway.
2. Secret Manager + Key Manager propres.
3. Container Registry.
4. Serverless Jobs.
5. Queues / RabbitMQ.
6. Edge Services.
7. Revue Kubernetes ou Bare Metal uniquement apres metriques production.

## Phase 1 - Securiser la fondation reseau

### Objectif

Faire entrer le trafic public par un point controle et retirer l'exposition directe des instances applicatives quand Scaleway et les besoins d'administration le permettent.

### Scope

- Ajouter un Load Balancer devant `account-service` et `cloud-service`.
- Retirer l'exposition directe des instances applicatives quand possible.
- Ajouter un Public Gateway pour que les instances privees gardent une sortie internet controlee.
- Separer explicitement les zones reseau:
  - subnet public: Load Balancer et Public Gateway;
  - subnet prive: API, workers, PostgreSQL, Redis eventuel.
- Revoir les security groups:
  - SSH tres restreint;
  - HTTP/HTTPS uniquement via Load Balancer;
  - PostgreSQL et Redis accessibles seulement depuis le subnet prive ou les groupes autorises.

### Livrable

- Terraform reseau propre.
- Outputs DNS et Load Balancer.
- Documentation des flux entrants/sortants.

### Critere d'acceptation

Une instance applicative peut etre remplacee sans modifier le point d'entree public, et aucune API production n'est exposee directement hors Load Balancer.

## Phase 2 - Industrialiser secrets et cles

### Objectif

Sortir les secrets applicatifs des VM hors bootstrap minimal et rendre les acces, rotations et procedures d'urgence explicites.

### Scope

- Provisionner les secrets applicatifs dans Secret Manager via Terraform ou procedure controlee:
  - `NVBES_DATABASE_URL`;
  - `NVBES_JWT_SECRET`;
  - secrets Stripe;
  - `SCW_TEM_*`;
  - credentials Redis;
  - secret E2EE.
- Standardiser le nommage:
  - `nvbes/staging/account-service/NVBES_JWT_SECRET`;
  - `nvbes/prod/cloud-service/STORAGE_BUCKET`.
- Activer Key Manager pour les secrets ou operations cryptographiques critiques.
- Documenter:
  - rotation;
  - acces IAM;
  - break-glass infra;
  - responsabilites CI/CD versus runtime.

### Livrable

Runtime sans secrets statiques sur VM, hors bootstrap minimal documente.

### Critere d'acceptation

Un secret applicatif peut etre remplace sans rebuild d'image et sans modifier manuellement une VM durable.

## Phase 3 - Normaliser les artefacts de deploiement

### Objectif

Faire de Container Registry la source unique des images deployables.

### Scope

- Ajouter Container Registry pour:
  - `account-service`;
  - `cloud-service`;
  - `account-worker`;
  - `cloud-worker`.
- Adapter CI/CD pour build, tag et push.
- Deployer les instances depuis images versionnees ou pulls controles.
- Definir une politique de retention registry.

### Livrable

Deploiements reproductibles par version d'image.

### Critere d'acceptation

La version en staging et la version en production sont identifiables par digest ou tag immuable, avec rollback documente.

## Phase 4 - Sortir les jobs batch des workers permanents

### Objectif

Reduire la charge et la complexite des workers permanents en deplacant les batchs non temps reel vers Serverless Jobs.

### Scope

- Identifier les taches batch:
  - exports RGPD;
  - cleanup storage;
  - scans fichiers;
  - maintenance multipart uploads;
  - rapports d'audit;
  - sync et billing jobs.
- Migrer les taches non temps reel vers Serverless Jobs.
- Garder les workers pour le traitement continu ou faible latence.
- Ajouter logs, retry, timeout et idempotency.

### Livrable

Baisse de charge worker et batchs isoles, observables et relancables.

### Critere d'acceptation

Un batch echoue est visible, rejouable et n'impacte pas la capacite des workers temps reel.

## Phase 5 - Remplacer ou durcir les queues Redis

### Objectif

Eviter que Redis porte des workflows critiques sans garanties de recuperation adaptees.

### Scope

- Classer les queues actuelles:
  - critique business;
  - email;
  - billing/webhooks;
  - scan/storage;
  - maintenance.
- Choisir la cible par type:
  - Queues / Topics & Events pour evenementiel simple;
  - RabbitMQ pour workflows complexes avec retry, DLQ et routing;
  - Redis uniquement pour cache, session ou queues non critiques.
- Ajouter DLQ, retry policy et metriques de profondeur.
- Adapter progressivement les workers.

### Livrable

Jobs critiques fiables, auditables et recuperables.

### Critere d'acceptation

Une panne worker ne provoque pas de perte silencieuse d'un job critique, et les jobs bloques ont une procedure de diagnostic.

## Phase 6 - Optimiser Drive avec Edge Services

### Objectif

Ameliorer les downloads Drive publics ou semi-publics sans perturber les flux upload existants.

### Scope

- Mettre Edge Services devant Object Storage pour les downloads publics ou semi-publics.
- Tester:
  - cache;
  - WAF;
  - headers;
  - compatibilite signed URL.
- Garder les uploads directs sur Object Storage si le flux TUS/S3 actuel reste plus robuste sans edge.
- Mesurer cout, latence et cache hit ratio.

### Livrable

Downloads Drive plus rapides et mieux proteges.

### Critere d'acceptation

Edge Services ameliore la latence ou la protection mesurablement sans casser les signed URLs ni le controle d'acces.

## Decisions reportees

A ne pas faire maintenant sans besoin clair:

- Kubernetes: trop lourd pour le stade actuel.
- Bare Metal: utile seulement avec contrainte forte de cout, performance ou isolation.
- File Storage / Block Storage: non prioritaire tant que Drive reste oriente Object Storage.
- Functions: moins naturel que Serverless Jobs ou Containers pour le backend Rust.
- Generative APIs: a traiter comme feature produit, pas comme infrastructure de base.

## Definition de done globale

- Le changement est decrit en Terraform ou dans une procedure controlee.
- Les outputs utiles a l'exploitation sont exposes.
- Les secrets reels ne sont jamais commits.
- Les flux reseau et IAM sont documentes.
- Les checks cibles de l'infrastructure modifiee sont executables localement.
- Le produit Scaleway ajoute retire une responsabilite operationnelle explicite.
