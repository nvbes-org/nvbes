# Stratégie de test Billing

> **Statut : socle V1 actif, mode test Stripe uniquement.** Le billing-service ne
> traite que du test mode Stripe. Les webhooks sont vérifiés par signature Stripe,
> les événements livemode sont rejetés, le traitement est idempotent (advisory locks)
> et les échecs partent en dead-letter de réconciliation.

## Portée et statut de preuve

Cette stratégie couvre `apps/billing-service` et `nvbes-billing` (vérification de
signature Stripe, mapping des statuts d'abonnement). Billing est un runtime HTTP
complémentaire d'Identity (les scopes `billing:read`/`billing:write` viennent du JWT
Identity vérifié localement).

Routes réelles :

| Méthode  | Route                                                                             | Rôle                                                    |
| -------- | --------------------------------------------------------------------------------- | ------------------------------------------------------- |
| GET      | `/health/live`, `/health/ready`                                                   | sondes                                                  |
| GET      | `/metrics`                                                                        | Prometheus (Bearer)                                     |
| POST     | `/webhooks/stripe`                                                                | signature Stripe, rejet livemode, traitement idempotent |
| GET      | `/billing/plans`                                                                  | catalogue de plans public                               |
| POST     | `/workspaces/{id}/billing/checkout`                                               | création session checkout (`billing:write`)             |
| POST     | `/workspaces/{id}/billing/portal`                                                 | session portal (`billing:read`)                         |
| GET      | `/workspaces/{id}/billing/overview`                                               | état d'abonnement (`billing:read`)                      |
| GET/POST | `/operator/billing/overview`, `/reconciliations`, `/reconciliations/{id}/resolve` | opérateur solo                                          |

Événements Stripe traités : `checkout.session.completed`,
`customer.subscription.created/updated/deleted`, `invoice.paid`, `invoice.payment_failed`.

## État d'automatisation réel

| Lane             | Déclenchement réel                     | Couverture actuelle                                                                            | Preuve             |
| ---------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------ |
| PR               | CI du dépôt                            | `cargo test --locked` : auth, config, health; `billing.webhooks.tests.rs` (feature DB)         | rapports Cargo     |
| Database         | lane `database` CI (PostgreSQL isolé)  | `billing.database.tests.rs`, `billing.webhooks.tests.rs` (signature, idempotence, dead-letter) | `test:database` Nx |
| Workflow sandbox | `.github/workflows/stripe-sandbox.yml` | préflight/sandbox Stripe test en CI                                                            | journal sandbox    |
| Containers       | lane `containers` CI                   | contrat conteneur : reproduisibilité, non-root, migrations avant serveur, smoke billing        | `test:contract` Nx |
| Trusted hermetic | commande opérateur                     | PostgreSQL éphémère + Identity + Billing isolés, `synthetic-billing-smoke`                     | journaux signés    |
| Pre-release      | session signée                         | campagnes sur settings Stripe test, réconciliation opérateur, scenarii d'abonnement            | rapports signés    |

Le workflow `stripe-sandbox.yml` valide la configuration Stripe test sans frais réels.
Les jobs billing ne produisent jamais de workflow vert en étant silencieusement ignorés.

### Limites explicites

- Aucun traitement livemode : les événements livemode sont rejetés à l'ingestion,
  vérifié par test. La compatibilité livemode est un gap volontaire V1 (cible B2C
  progressive, pas d'offre B2B).
- La réconciliation est un process opérateur semi-manuel : les événements dead-letterés
  dans `billing_reconciliation_items` sont résolus via `/operator/.../resolve`. Pas
  d'automatisation tant qu'un besoin mesuré ne la justifie pas.
- Aucune campagne de charge k6 billing dédiée : la charge passe par les plans globaux
  s'ils existent; Billing n'a pas de profil k6 propre.
- Les scopes `billing:read`/`billing:write` sont vérifiés via Identity; l'isolation
  tenant/workspace reste couverte au niveau integration DB, pas par un scan séparé.
- Le checkout est hors UI : aucun parcours Playwright de checkout n'est branché V1.

## Fiabilité et sécurité du harness

- La signature Stripe est vérifiée avec l'implémentation réelle de `nvbes-billing`
  (HMAC, timestamp tolerance). Le keyword `billing.verify_signature` reproduit cette
  vérification hors-ligne pour les cases YAML.
- Les événements de test ont `livemode: false`; un événement livemode doit être rejeté
  (test dédié).
- L'idempotence repose sur des advisory locks PostgreSQL : rejouer un même événement ne
  crée pas de doublon. Le keyword `billing.check_idempotence` rejoue le même événement
  signé et vérifie l'absence de double traitement.
- Aucun appel Stripe en PR; les tests d'intégration Stripe passent par la sandbox test.
- Les artefacts sont rédigés : pas de clé Stripe, pas de secret de webhook.

## Environnement et données de test

- PostgreSQL éphémère migré depuis zéro (1 migration) pour la lane database.
- Variables requises : clés Stripe test, `STRIPE_WEBHOOK_SECRET` de test,
  URLs Identity pour la validation des scopes JWT.
- Commandes CLI : `migrate`, `validate-runtime`, `synthetic-billing-smoke`.
- Budget : le runtime Billing est couvert par le plafond global de 30 EUR TTC/mois;
  aucun appel Stripe produisant de coût réel.

## Suites ISO 29119-3

Suites du §3.2 de `03-documentation.md` :

| Suite ID            | Catégorie | Commande réelle                                               |
| ------------------- | --------- | ------------------------------------------------------------- |
| TS-BILLING-WEBHOOKS | webhook   | `cargo test --package nvbes-billing-service billing.webhooks` |

Test cases `TC-BILLING-WEBHOOKS-<NUM>` : signature valide, signature invalide,
livemode rejeté, événement inconnu, rejeu idempotent, dead-letter d'un événement en
échec. Le runner keyword-driven couvre la préparation : `infra.setup_db`,
`infra.migrate`, `infra.start_service`, `infra.health_check` puis les keywords
`billing.create_webhook`, `billing.verify_signature`, `billing.process_event`,
`billing.check_idempotence`.

## Gates et acceptation

- PR : lint, check, unitaires, contrats conteneur, migrations, sandbox Stripe.
- Avant staging : `synthetic-billing-smoke` vert sur l'environnement déployé; settings
  Stripe test vérifiés.
- Avant production : réconciliation à jour (aucun événement dead-letteré non résolu),
  webhooks test verts, approval explicite signée.
- Tout bug de facturation commence par un test rouge au plus bas niveau pertinent
  (signature, idempotence, mapping de statut).

## Principes de qualité

- Webhook signé + idempotent : aucune déduplication ne peut être assouplie sans test.
- Aucun événement livemode ne devient vert dans la suite.
- Pas de fausse déclaration d'automatisation : la réconciliation reste opérateur.
- Pas de secret Stripe ni webhook secret dans les artefacts CI.
- Seuils de durée s'ils existent sont des planchers versionnés.
