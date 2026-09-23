# Stratégie de test Platform Operations

> **Statut : socle V1 actif (opérateur solo).** Le platform-operations-service est le
> cockpit de l'opérateur : cases durables, trail d'audit append-only, ledger de coûts
> FinOps, agrégation de la santé des services et procédures en mode dégradé. La logique
> vit dans `libs/rust/platform` (`nvbes-platform`); l'app de `apps/` n'est qu'un shim.

## Portée et statut de preuve

Cette stratégie couvre le tree `platform` (app + crate `nvbes-platform`) : routes,
actions, FinOps, degraded, health, operations cases, audit, outbox. Les mutations
métier par `/api/v1/actions` sont **intentionnellement indisponibles** (réponse 501) :
les mutations passent par `/api/v1/commands` et le pattern case/audit.

Routes réelles :

| Méthode | Route                                 | Rôle                                      |
| ------- | ------------------------------------- | ----------------------------------------- |
| GET     | `/health/live`, `/health/ready`       | sondes (ready requête `operations_audit`) |
| GET     | `/`, `/cockpit`                       | cockpit opérateur (JWT oplog)             |
| GET     | `/api/v1/overview`                    | vue d'ensemble (JWT oplog)                |
| POST    | `/api/v1/actions`                     | **506/501** : indisponible par design     |
| POST    | `/api/v1/commands`                    | mutation (scope WriteCases)               |
| GET     | `/api/v1/cases`, `/api/v1/cases/{id}` | cases (ReadCases)                         |
| GET     | `/api/v1/audits`                      | trail d'audit (ReadAudit)                 |
| GET     | `/api/v1/costs`                       | ledger de coûts (ReadAudit)               |

## État d'automatisation réel

| Lane        | Déclenchement réel                             | Couverture actuelle                                                                                                                                                                                                       | Preuve             |
| ----------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| PR          | CI du dépôt                                    | `cargo test --package nvbes-platform` : cockpit.server, cockpit.health, cockpit.actions, cockpit.finops, finops.budget, operations, event, outbox + 6 modules inline (auth, backup, degraded, trust_risk, billing, email) | rapports Cargo     |
| Database    | `scripts/test-platform-operations-database.sh` | test DB dédié (target `test:database`)                                                                                                                                                                                    | script DB          |
| Containers  | lane `containers` CI                           | contrat conteneur : reproduisibilité, non-root, migrations avant serveur                                                                                                                                                  | `test:contract` Nx |
| Pre-release | session signée                                 | scenarii opérateur : cases, audits, coûts, mode dégradé, agrégation de santé                                                                                                                                              | rapports signés    |

### Limites explicites

- Les actions métier sont 501 par design; la stratégie **teste que la mutation par
  actions reste impossible** et que `/api/v1/commands` est le seul chemin de mutation.
- Le ledger de coûts (FinOps) est calculé à partir des données accumulées; sa
  correction est vérifiée en DB et par les tests `finops.budget`, pas par une preuve
  externe de facturation.
- Le mode dégradé a un module de tests inline; la procédure complète
  (exécution opérateur réelle en environnement de test) reste à cadrer en procédures.
- L'agrégation de santé agrège les sondes des autres services; elle est testée avec
  un stub, pas contre les services réels en chaque PR.
- Pas de campagne UI : le cockpit est en HTTP JSON, pas de frontend branché V1.

## Fiabilité et sécurité du harness

- L'audit est append-only : les tests `platform.audit` et `operations` vérifient
  qu'aucun événement n'est réécrit/supprimé.
- cases/commands sont testés avec le pattern case→audit; aucun commande non autorisée
  (scope JWT) ne passe.
- Le FinOps gate (30 EUR TTC/mois) est un test : `finops.budget` et `cockpit.finops`
  vérifient le ledger et le budget; un écart de coût casse un test, pas un commit.
- L'authentification cockpit (JWT oplog) est testée; aucune route hors health n'est
  accessible sans JWT.
- Les artefacts sont rédigés : pas de secret oplog ni de données opérateur réelles.

## Environnement et données de test

- PostgreSQL éphémère avec la migration dédiée `platform` (audit, cases, ledger) via
  `scripts/test-platform-operations-database.sh`.
- La cible `test:database` doit produire un rapport publiable; le script isole le
  PostgreSQL de test (loopback, non production).
- Commandes CLI : `migrate`, `validate-runtime`, `synthetic-*` si présents dans
  `nvbes-platform`, sinon le contrat conteneur et le rapport DB font preuve.
- Budget : port 8084, `min_scale = 0` — service on-demand, coût couvert par le plafond
  global de 30 EUR TTC/mois.

## Suites ISO 29119-3

Suites du §3.2 de `03-documentation.md` | Commande réelle :

| Suite ID            | Catégorie | Commande réelle                                                                  |
| ------------------- | --------- | -------------------------------------------------------------------------------- |
| TS-PLATFORM-COCKPIT | existence | `cargo test --package nvbes-platform platform.cockpit` (auth, server, ServiceId) |
| TS-PLATFORM-FINOPS  | existence | `cargo test --package nvbes-platform platform.finops`                            |
| TS-PLATFORM-AUDIT   | existence | `cargo test --package nvbes-platform platform.audit`                             |
| TS-PLATFORM-OPS     | security  | `cargo test --package nvbes-platform platform.operations`                        |

Les anciens modules cockpit simulés (health aggregator, email/billing/finops
panels, degraded dispatcher, actions métier) ont été retirés : l'API live ne
sert que health, overview minimal, commands/cases/audits/costs, et 501 sur
`/api/v1/actions`. Les procédures dégradées restent dans le runbook et les
mots-clés `platform.degraded_procedure` du test-utils.

## Gates et acceptation

- PR : lint, check, unitaires `nvbes-platform`, contrats conteneur, migrations.
- Avant staging : health aggregator vert en environnement de test.
- Avant production : audit vide de tout événement non tracé, aucun case bloquant
  non résolu, FinOps gate vert (budget ≤ 30 EUR TTC/mois), approval explicite signée.
- Toute dérive de coût casse `finops.budget` avant tout déploiement.

## Principes de qualité

- Append-only : l'audit ne se réécrit pas, les tests le garantissent.
- Le FinOps est un gate de livraison, pas un tableau de bord optionnel.
- Le 501 des actions métier est un comportement testé, pas un défaut.
- Aucune route cockpit hors health sans JWT opérateur.
- La plateforme reste on-demand (`min_scale = 0`) et dans le plafond de 30 EUR TTC/mois.
