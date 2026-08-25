# Fondation FinOps nvbes V1 — registre d’exécution

## Statut

Implémentation terminée le 25 août 2026 sur la branche
`codex/v1-finops-foundation`.

Ce document est le plan exécuté et le registre de preuve de la fondation
FinOps V1. Il complète la
[conception validée](../specs/2026-08-25-nvbes-v1-finops-platform-operations-design.md)
sans autoriser une mise en production ni introduire un produit utilisateur.

## Objectif

Faire du coût une propriété vérifiable du socle nvbes avant le premier produit :

- cible mensuelle de production de **20 EUR TTC** ;
- limite mensuelle absolue de **30 EUR TTC** ;
- capacité serverless Scaleway à descendre à zéro ;
- capacité maximale explicitement bornée à une unité par runtime V1 ;
- politique de dégradation réutilisable par les futurs services ;
- contrôles automatiques locaux et CI sans abonnement supplémentaire ;
- factures et estimations fournisseur TTC comme autorité opérationnelle tant
  qu’aucune collecte de dépense live n’existe.

La V1 reste un socle B2C réutilisable pour Identity, Account, Billing, Email,
Trust/Risk et les futures opérations de plateforme. Les équipes sont prises en
compte dans les frontières, mais les fonctions Enterprise et la conformité B2B
restent futures.

## Principes conservés

- Aucune dette structurelle connue n’est acceptée sur les zones touchées.
- Toute dette future est enregistrée, évaluée, attribuée et traitée
  proactivement.
- Une automatisation n’est retenue que si elle protège une propriété critique,
  réduit un volume manuel mesuré ou empêche un dépassement budgétaire.
- Les opérations manuelles restent le défaut lorsqu’une automatisation sûre ne
  tient pas dans le budget global.
- Une solution maison reste recevable si elle est moins coûteuse, sûre,
  maintenable et remplaçable ; le parsing HCL n’entre pas dans cette catégorie,
  car une bibliothèque maintenue existe.
- La mise en production devra être progressive, mesurée et réversible.

## Architecture livrée

```text
production-budget.json
        │
        ├── validation et sélection de palier JavaScript
        ├── gate racine `pnpm check:finops`
        └── procédure opérateur documentée

fichiers Terraform `.tf`
        │
        └── parseur HCL maintenu → contrôle des attributs directs → gate FinOps

future dépense live TTC
        │
        └── BudgetThresholds Rust → BudgetStage → actions du domaine propriétaire
```

La livraison contient uniquement des contrats, contrôles, plafonds
d’infrastructure et primitives de bibliothèque. **Aucun runtime produit,
collecteur de coût, backoffice ou automatisation d’alerte n’a été déployé.**

## Hors périmètre

- Cloud, Drive ou tout autre produit utilisateur final ;
- déploiement d’un service Platform Operations ;
- collecte automatique des factures ou dépenses live ;
- application automatique des paliers de dégradation ;
- alertes fournisseur considérées comme mécanisme d’enforcement ;
- fonctionnalités Enterprise, contrats B2B ou conformité avancée ;
- modération propre à un produit encore inexistant ;
- haute disponibilité permanente, multi-région ou nouveaux composants payants.

## Contrat budgétaire exécutable

Le fichier
[`production-budget.json`](../../../infrastructure/finops/production-budget.json)
est la source versionnée du budget mensuel de production. Tous les montants sont
en centimes d’euro, taxes incluses.

| Propriété | Valeur |
| --- | ---: |
| Cible mensuelle | 2 000 (20 EUR) |
| Limite mensuelle | 3 000 (30 EUR) |
| Réserve entre cible et limite | 1 000 (10 EUR) |
| Alertes informatives | 1 500, 2 000, 2 500, 2 800, 3 000 |

### Catégories

| Catégorie | Cible TTC | Limite TTC |
| --- | ---: | ---: |
| `domain_dns` | 200 | 300 |
| `compute` | 300 | 600 |
| `postgres` | 400 | 800 |
| `email` | 100 | 200 |
| `storage_registry` | 200 | 400 |
| `observability` | 0 | 200 |
| `safety_margin` | 800 | 800 |
| **Total** | **2 000** | **3 300** |

Les cibles de catégorie totalisent exactement 20 EUR. La somme des limites de
catégorie peut dépasser 30 EUR pour permettre une réallocation mesurée, mais le
plafond mensuel global de 30 EUR reste impératif et ne peut être relevé sans
nouvelle décision de conception.

### Paliers de dégradation

| Dépense TTC constatée | `BudgetStage` | Comportement attendu |
| ---: | --- | --- |
| 0 à 2 499 | `normal` | Travail V1 budgété disponible |
| Dès 2 500 | `disable_non_essential` | Désactiver les traitements optionnels |
| Dès 2 800 | `freeze_cost_creation` | Refuser les nouvelles opérations créatrices de coût |
| Dès 3 000 | `essential_only` | Conserver uniquement récupération, email critique et ingestion durable des webhooks fournisseur |

Les seuils peuvent être avancés par configuration Rust pour agir plus tôt. Ils
ne peuvent jamais dépasser les maxima V1 respectifs de **2 500**, **2 800** et
**3 000** centimes.

## Lots exécutés

### 1. Contrat de budget

Fichiers :

- [`tools/finops/production-budget.mjs`](../../../tools/finops/production-budget.mjs) ;
- [`tools/finops/production-budget.test.mjs`](../../../tools/finops/production-budget.test.mjs) ;
- [`infrastructure/finops/production-budget.json`](../../../infrastructure/finops/production-budget.json).

Comportements livrés : sept clés de catégorie exactes, entiers non négatifs en centimes,
alertes strictement croissantes, dernière alerte égale à la limite, cibles totalisant
20 EUR, sélection déterministe du palier et chargement du contrat réel. Le validateur
n’affirme pas rejeter les propriétés supplémentaires hors de ces invariants documentés. **9 tests passent.**

Critère de sortie : un contrat invalide échoue avant tout changement
d’infrastructure.

### 2. Gate FinOps racine

Fichiers :

- [`tools/finops/check-production-budget.mjs`](../../../tools/finops/check-production-budget.mjs) ;
- [`tools/ci/finops-workflows-contract.test.mjs`](../../../tools/ci/finops-workflows-contract.test.mjs) ;
- [workflows CI et déploiement](../../../.github/workflows) ;
- [`package.json`](../../../package.json).

`pnpm check:finops` enchaîne le contrat budgétaire, les plafonds Terraform et le contrat
des workflows. Le `pnpm check` racine l’appelle. Le workflow CI et les deux workflows de
déploiement l’exécutent exactement une fois, après l’installation pnpm verrouillée et avant
la première commande Terraform, dans `email-quality` ou `ci-test-gate`. Les jobs payants restent protégés par `needs`.

Critère de sortie : aucun changement concerné ne peut être déclaré valide si le
budget ou une borne de capacité échoue.

### 3. Analyse HCL des bornes serverless

Fichiers :

- [`tools/finops/check-scale-to-zero.mjs`](../../../tools/finops/check-scale-to-zero.mjs) ;
- [`tools/finops/check-scale-to-zero.test.mjs`](../../../tools/finops/check-scale-to-zero.test.mjs) ;
- [`package.json`](../../../package.json) ;
- [`pnpm-lock.yaml`](../../../pnpm-lock.yaml).

Le contrôle utilise exclusivement **`@cdktn/hcl2json@0.24.0`**, version épinglée.
Il découvre récursivement et trie les fichiers `.tf`, ignore `.terraform`, puis
parse chaque fichier séparément. Il inspecte uniquement les attributs directs
des ressources :

- `scaleway_container` exige `min_scale = 0` et `max_scale` égal à 0 ou 1 ;
- `scaleway_sdb_sql_database` exige `min_cpu = 0` et `max_cpu` égal à 0 ou 1.

Les valeurs admises sont les nombres sémantiques 0 et 1 produits par le
parseur. Les expressions, chaînes, attributs manquants et `-0` sont rejetés. Les
commentaires, blocs imbriqués, templates, heredocs, CRLF, erreurs de syntaxe et
l’isolation entre ressources sont couverts. **11 tests passent.**

Le contrôle n’utilise **jamais de regex pour parser HCL**.

Critère de sortie : toute ressource ciblée est parseable, peut descendre à zéro
et possède un maximum littéral ne dépassant pas une unité.

### 4. Plafonnement Terraform actif

Fichiers :

- [`email-delivery.tf`](../../../infrastructure/environments/email-production/email-delivery.tf) ;
- [`runtime.tf`](../../../infrastructure/environments/trust-risk-production/runtime.tf) ;
- [`database.tf`](../../../infrastructure/environments/trust-risk-production/database.tf).

Les deux blocs de ressources conteneur ciblés ont `min_scale = 0` et `max_scale = 1` :
`email_runtime`, dont le `for_each` crée les rôles `ingress` et `dispatch`, et le runtime
Trust/Risk. La base Trust/Risk a `min_cpu = 0` et `max_cpu = 1`. Le stack Email historique
sous `infrastructure/stacks/email` est distinct et déjà borné ; aucun maximum ciblé supérieur à 1 ne subsiste.

Critère de sortie : le contrôle HCL passe sur l’arbre `infrastructure` réel.

### 5. Primitive Rust de politique budgétaire

Fichiers :

- [`platform.finops.budget.rs`](../../../libs/rust/platform/src/platform.finops.budget.rs) ;
- [`platform.finops.budget.tests.rs`](../../../libs/rust/platform/src/platform.finops.budget.tests.rs) ;
- [`lib.rs`](../../../libs/rust/platform/src/lib.rs).

`BudgetStage` est sérialisé en `snake_case`. `BudgetThresholds::try_new` exige
des seuils non nuls, strictement croissants et au plus égaux aux maxima V1
2 500/2 800/3 000. Des seuils anticipés sont permis. `stage_for` sélectionne le
palier de façon déterministe aux frontières et au-delà de la limite.

Critère de sortie : la primitive est réexportée par `nvbes-platform`, testée et
ne dépend d’aucun runtime ou fournisseur.

### 6. Procédure opérateur

Fichiers :

- [`infrastructure/finops/README.md`](../../../infrastructure/finops/README.md) ;
- [`infrastructure/README.md`](../../../infrastructure/README.md).

La procédure impose une revue TTC, l’attribution à une catégorie, l’exécution
du gate et une justification mesurée pour toute réallocation. Les alertes de
dépense proposées par les fournisseurs sont **informatives** : elles devront
être configurées et vérifiées manuellement avant chaque mise en production,
mais ne constituent ni un contrôle automatique livré ni l’autorité budgétaire.

Jusqu’à l’implémentation d’une collecte live, les **factures TTC et estimations
fournisseur vérifiées manuellement sont l’autorité**. Les plafonds Terraform
sont le garde-fou automatique de capacité.

## Décisions et déviations d’exécution

### Remplacement du scanner artisanal

Le premier plan décrivait un scanner textuel maison. Les revues adversariales
ont démontré qu’un parseur incomplet pouvait emprunter des accolades situées
dans des templates ou heredocs et accepter ou refuser la mauvaise ressource.
Les correctifs successifs sur commentaires et chaînes n’éliminaient pas cette
classe de contournement.

Décision finale : supprimer intégralement le scanner custom et utiliser le fork
maintenu `@cdktn/hcl2json@0.24.0`. Cette déviation ferme les bypass de templates,
heredocs et blocs imbriqués, réduit le code propriétaire et ne crée aucun coût
récurrent. Elle est cohérente avec l’exigence de zéro dette technique.

### Autorité des coûts

Le plan initial supposait des alertes fournisseur automatiques. Aucune collecte
live ni vérification multi-fournisseur n’est livrée. Présenter ces alertes comme
automatiques aurait créé une garantie fictive.

Décision finale : les alertes restent informatives et manuellement
configurées/vérifiées. La facture ou l’estimation TTC manuelle prévaut jusqu’à
ce qu’un futur runtime collecte et rapproche effectivement les coûts.

## Validation reproductible

Depuis la racine du dépôt :

```bash
pnpm check:finops
node --test tools/finops/production-budget.test.mjs
node tools/finops/check-production-budget.mjs
node --test tools/finops/check-scale-to-zero.test.mjs
node tools/finops/check-scale-to-zero.mjs infrastructure
node --test tools/ci/finops-workflows-contract.test.mjs
cargo test -p nvbes-platform
cargo check --workspace --locked
```

Pour un changement Terraform ciblé, exécuter aussi le formatage Terraform du
répertoire concerné avant le gate. Pour un changement transversal, conserver
les checks racine imposés par le dépôt.

## Historique signé pertinent

Tous les commits ci-dessous ont une signature GPG valide dans l’historique
local (`%G? = U` : signature valide, clé non marquée comme explicitement fiable
par le trust store local).

| Commit | Objet |
| --- | --- |
| `5b0cd983` | Conception FinOps et Platform Operations validée |
| `17627af` | Plan d’implémentation initial |
| `9c5ce450`, `8d95d4bb` | Contrat budgétaire et alignement du schéma |
| `3b623117` | Intégration au gate racine |
| `bfa40644`, `1fc7cfc7` | Parseur HCL maintenu et assertions complètes |
| `f22fbfd6` | Plafonds des runtimes Terraform actifs |
| `9eda4d23`, `8b00671e` | Primitive Rust et enforcement des maxima V1 |
| `c39f65a2`, `7f19a962` | Procédure opérateur et autorité manuelle clarifiée |
| `fc0416a` | Enforcement CI du gate FinOps avant déploiement |

Les commits intermédiaires `59d0b1cc`, `5d90eee5`, `7764c9f8`, `4f1a220d` et
`34979bc5` documentent la tentative de scanner maison. Leur implémentation est
entièrement remplacée par `bfa40644` et ne constitue pas l’état final.

## Critères de complétion

- [x] Budget TTC versionné : cible 20 EUR, limite 30 EUR.
- [x] Catégories, alertes et paliers validés automatiquement.
- [x] Contrat couvert par 9 tests.
- [x] Parse HCL réel, sans regex, version de dépendance épinglée.
- [x] Contrôle des bornes couvert par 11 tests.
- [x] Toutes les ressources Scaleway ciblées descendent à zéro et plafonnent à 1.
- [x] Maxima Rust V1 impossibles à repousser au-delà de 2 500/2 800/3 000.
- [x] Seuils Rust anticipés permis.
- [x] Procédure manuelle TTC et statut informatif des alertes documentés.
- [x] Aucun runtime produit ou collecteur de dépense ajouté.
- [x] Gate `pnpm check:finops` vert sur l’arbre réel.
- [x] Gate FinOps exécuté avant Terraform par les trois workflows concernés.

## Condition d’ouverture future

Cette fondation ne suffit pas à ouvrir un produit. Avant toute production, il
faudra au minimum vérifier manuellement les estimations TTC, configurer et
tester les alertes fournisseur informatives, exécuter les gates, documenter le
rollback et effectuer un déploiement progressif. Toute automatisation live des
coûts devra consommer ces contrats sans en affaiblir les plafonds.
