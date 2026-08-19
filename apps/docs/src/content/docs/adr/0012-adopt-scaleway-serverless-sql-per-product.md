---
title: "ADR 0012 - Adopter Scaleway Serverless SQL par produit"
description: Architecture Decision Record - nvbes platform
---

## Status

Accepted.

## Date

2026-08-02

## Context

nvbes sépare Identity, Account, Cloud, Billing, Email, Developer, Enterprise et
Backoffice en produits autonomes. Chaque produit doit posséder ses données et
ses credentials sans partager directement les tables d'un autre produit.

Déployer une instance PostgreSQL permanente par produit préserverait ces
frontières, mais imposerait un coût fixe disproportionné pendant le
développement, le staging et la V0, lorsque la majorité des bases restent
inactives une grande partie du mois. Mutualiser une base ou des credentials
entre produits réduirait ce coût au prix d'un couplage opérationnel et d'une
frontière d'autorisation plus faible.

Au tarif étudié, Scaleway Serverless SQL facture environ
`0,13572 €/vCPU/heure` et `0,000272 €/Go/heure`. Avec `min_vCPU = 0`, la base
peut passer automatiquement à zéro après cinq minutes sans requête. Le coût
estimé devient alors :

| Solution | Coût mensuel approximatif HT |
|---|---:|
| Serverless SQL inactive, 1 Go | 0,20 € |
| Serverless SQL, 10 heures actives et 10 Go | 3,35 € |
| Serverless SQL, 50 heures actives et 10 Go | 8,78 € |
| PostgreSQL managé `DB-DEV-S` et 10 Go | 12,70 € |
| PostgreSQL auto-hébergé sur `PLAY2-PICO` | 10,42 €, hors stockage et sauvegardes |

Une instance managée `DB-DEV-S` coûte environ `0,0156 €/heure`, soit
11,39 € par mois avant stockage. À tarifs et volumes comparables, elle devient
plus économique qu'une base Serverless SQL lorsque celle-ci reste active
environ 80 heures par mois.

Serverless SQL présente aussi des différences avec une instance PostgreSQL
classique. En particulier, les advisory locks ne sont pas garantis à cause du
pooling. Le dépôt utilise actuellement `pg_advisory_xact_lock`; conserver ce
mécanisme rendrait certains chemins incompatibles avec la cible retenue.

Références :

- [tarification Scaleway des bases de données](https://www.scaleway.com/en/pricing/managed-databases/) ;
- [fonctionnement de Serverless SQL et mise en veille](https://www.scaleway.com/en/docs/serverless-sql-databases/reference-content/serverless-sql-databases-overview/) ;
- [différences connues avec PostgreSQL](https://www.scaleway.com/en/docs/serverless-sql-databases/reference-content/known-differences/).

## Decision

Utiliser Scaleway Serverless SQL comme cible PostgreSQL par défaut pour le
développement, le staging et les produits V0 dont l'activité reste
intermittente.

Créer une base distincte pour chacun des produits suivants :

- Identity ;
- Account ;
- Cloud ;
- Billing ;
- Email ;
- Developer ;
- Enterprise ;
- Backoffice.

Chaque service possède des credentials IAM propres et ne peut accéder qu'à sa
base. Les migrations, sauvegardes, métriques de consommation et procédures de
restauration restent également attribuées au produit propriétaire.

Configurer chaque base avec `min_vCPU = 0`. Le calcul maximal est plafonné par
produit afin de limiter les dépenses imprévues. Un plafond différent exige une
justification fondée sur la charge mesurée.

Configurer les pools SQLx avec `min_connections = 0`, une durée d'inactivité
courte et une fermeture effective des connexions inutilisées. Les services ne
maintiennent pas une connexion uniquement pour réduire une éventuelle latence
de réveil.

Les probes de l'orchestrateur vérifient la disponibilité du processus sans
interroger PostgreSQL continuellement. Aucun ping périodique, métrique active,
job de maintenance ou cron ne doit empêcher la mise en veille. Les travaux
périodiques sont regroupés lorsque cela ne compromet pas leurs garanties
métier.

Un produit migre individuellement vers PostgreSQL managé `DB-DEV-S`, ou vers
une classe managée adaptée à sa charge, lorsque les mesures montrent que sa
base dépasse durablement environ 80 heures actives par mois. Le seuil est
réévalué avec les tarifs, le stockage, les exigences de disponibilité et les
besoins de performance réels. Identity est le candidat le plus probable à une
première migration, sans que cela impose de déplacer les autres produits.

Remplacer les usages de `pg_advisory_xact_lock` par des mécanismes PostgreSQL
compatibles avec Serverless SQL, selon l'invariant à protéger :

- contraintes uniques et opérations atomiques ;
- `SELECT ... FOR UPDATE` sur des lignes persistantes ;
- lignes de verrouillage explicites ;
- transactions courtes ;
- clés d'idempotence et résultats rejouables.

Le choix précis est fait par invariant métier. Une suppression mécanique des
locks sans garantie équivalente est interdite.

Ne pas auto-héberger PostgreSQL pour économiser environ deux euros par mois.
Les sauvegardes, restaurations, mises à jour de sécurité, incidents disque,
supervision et astreintes ne justifient pas cet écart.

Ne pas introduire un fournisseur de base externe à ce stade. Le gain potentiel
ne compense pas une dépendance, une latence inter-cloud, du trafic sortant et
une revue RGPD supplémentaires alors que Serverless SQL peut déjà approcher un
coût nul au repos.

## Consequences

- Les frontières de données et d'autorisation restent alignées sur les
  produits sans financer huit instances PostgreSQL permanentes.
- Huit bases inactives de 1 Go représentent environ 1,60 € HT par mois aux
  tarifs étudiés, hors options ou consommation additionnelle.
- Chaque produit peut évoluer et migrer vers une offre managée indépendamment
  des autres.
- L'hébergement applicatif et les bases restent chez Scaleway, ce qui réduit la
  latence inter-cloud et le nombre de sous-traitants à gouverner.
- La première requête après une période d'inactivité peut subir une latence de
  réveil. Les clients et timeouts doivent tolérer ce comportement.
- L'économie dépend de la capacité réelle de la base à dormir. Un health check
  SQL toutes les cinq minutes la maintiendrait active et porterait le seul
  calcul à environ 99 € HT par mois et par base.
- Les connexions persistantes, les métriques actives et les crons fréquents
  deviennent des éléments FinOps à contrôler explicitement.
- Les advisory locks ne peuvent plus servir de primitive de coordination. Leur
  remplacement augmente le travail initial mais rend les invariants plus
  explicites et plus robustes dans un environnement serverless ou poolé.
- Une base par produit augmente le nombre de migrations, secrets, tableaux de
  bord et exercices de restauration à administrer.
- Les estimations et le seuil de 80 heures dépendent des tarifs Scaleway et
  doivent être réévalués lorsqu'ils évoluent.

## Validation

La décision est correctement appliquée uniquement si :

- chaque produit possède une base et des credentials distincts ;
- une identité de service ne peut ni se connecter à la base d'un autre produit
  ni en lire les secrets ;
- `min_vCPU = 0` et un calcul maximal borné sont vérifiés dans l'infrastructure
  déployée ;
- tous les pools SQLx concernés utilisent `min_connections = 0` et libèrent les
  connexions inactives assez rapidement pour permettre la mise en veille ;
- les probes de l'orchestrateur n'exécutent aucune requête SQL périodique ;
- les métriques confirment qu'une base sans trafic atteint effectivement zéro
  après la fenêtre d'inactivité attendue ;
- les réveils, leur latence et leurs erreurs éventuelles sont observables ;
- aucun chemin compatible Serverless SQL ne dépend de
  `pg_advisory_xact_lock` ou d'un advisory lock équivalent non garanti ;
- les remplacements de locks sont couverts par des tests concurrents et des
  tests d'idempotence ;
- les heures actives, le stockage et le coût sont suivis séparément par
  produit ;
- une alerte déclenche une revue lorsque l'activité approche durablement le
  seuil économique de migration ;
- la restauration de chaque base est testée selon les objectifs du produit.
