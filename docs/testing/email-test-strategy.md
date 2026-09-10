# Stratégie de test Email

> **Statut : socle V1 actif.** L'email-worker est le runtime email : ingestion gRPC
> (`SubmitEmail`), dispatch via fournisseur (Scaleway TEM), réception de webhooks
> (topics/events), suppressions, rétention et opérations opérateur. Livraison
> at-least-once, pas exactly-once garantie.

## Portée et statut de preuve

Cette stratégie couvre `apps/email-worker` et `nvbes-email` (types de commande,
templates, provider client). C'est le service le plus testé du monorepo
(48 fichiers source, 18 blocs `#[cfg(test)]`) : l'axe fort de la preuve email est
l'automatisation unitaire + database.

Interfaces réelles :

| Protocole/Route                                        | Objet                                   |
| ------------------------------------------------------ | --------------------------------------- |
| HTTP `GET /health/live`, `/health/ready`               | sondes                                  |
| HTTP `GET /metrics`                                    | Prometheus (token observabilité)        |
| HTTP `POST /webhooks/scaleway/topics-and-events`       | webhooks SNS du fournisseur             |
| HTTP `POST /internal/queue/email-dispatch`             | trigger interne de dispatch             |
| HTTP `POST /internal/retention`                        | rétention (purge)                       |
| gRPC `nvbes.email.v1.EmailDeliveryService/SubmitEmail` | ingestion de commande                   |
| gRPC `EmailOperationsService`                          | snapshot, replay, suppressions, privacy |

Rôles runtime : `All`, `Ingress`, `Dispatch` — contrôle quelles routes sont montées.

## État d'automatisation réel

| Lane        | Déclenchement réel                    | Couverture actuelle                                                                                                                                                               | Preuve             |
| ----------- | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| PR          | CI du dépôt                           | unitaires système : config/retention, health, metrics, crypto, queue.trigger, webhook.verify, dispatch.attempt, dispatcher, grpc.auth, grpc.service.limits, main, synthetic_smoke | rapports Cargo     |
| Database    | lane `database` CI (PostgreSQL isolé) | webhook db, grpc.service, grpc.operations, dispatcher, operations.actions, operations.snapshot, database, dispatch.db, metrics.db, state, main (DB)                               | `test:database` Nx |
| Containers  | lane `containers` CI                  | contrat conteneur : reproduisibilité, non-root, migrations avant serveur, smoke                                                                                                   | `test:contract` Nx |
| Pre-release | session signée                        | scenarii opérateur via gRPC operations : snapshot, replay, suppressions, rétention                                                                                                | rapports signés    |

Test support dédié : `email.worker.test_support.rs` (builders config/state pour les
tests feature-gated). Pas de duplication de layout entre les plugins de test.

### Limites explicites

- La livraison externe réelle (Scaleway TEM) est un adapter : la preuve déclarée
  utilise un provider stub/test. Le webhook réel est réceptionné en test sur un
  endpoint local simulé; pas d'envoi réel en CI.
- Le contract de livraison est at-least-once : tests de rejeu, de dead-letter et de
  snapshots d'échec. Exactly-once est hors contrat.
- Les suppressions sont un process opérateur via gRPC operations; V1 ne les automatise
  pas.
- Pas de campagne UI : aucun frontend n'est branché V1 sur le worker.
- Le keyword HTTP `email.*` du runner couvre les sondes et le trigger interne; les
  flux métier (SubmitEmail, operations) reposent sur les tests gRPC unitaires et DB.

## Fiabilité et sécurité du harness

- Le webhook SNS est vérifié (signature/type d'événement) : les tests couvrent le
  content-type, l'event-type, et la signature invalide.
- Rétention testée (config et purge) avec données synthétiques; aucun email réel.
- Le dispatcher (canal mpsc) et la projection ont des tests unitaires et DB dédiés.
- `release-suppression <message-id> <actor> <reason>` : action opérateur tracée,
  jamais en PR.
- Les artefacts sont rédigés : aucune adresse réelle, clé de provider ou payload
  d'email dans les traces.

## Environnement et données de test

- PostgreSQL éphémère migré depuis zéro (4 migrations) pour la lane database et les
  tests feature-gated.
- Commandes CLI : `migrate`, `validate-runtime`, `error-reporting-smoke`,
  `synthetic-smoke`, `release-suppression`, `deployment-bootstrap`.
- Budget : worker email couvert par le plafond global de 30 EUR TTC/mois; le volume
  d'envois réels est conservateur et mesuré.

## Suites ISO 29119-3

Suites du §3.2 de `03-documentation.md` :

| Suite ID           | Catégorie | Commande réelle                                                   |
| ------------------ | --------- | ----------------------------------------------------------------- |
| TS-EMAIL-WEBHOOKS  | webhook   | `cargo test --package nvbes-email-worker email.worker.webhook`    |
| TS-EMAIL-DISPATCH  | dispatch  | `cargo test --package nvbes-email-worker email.worker.dispatcher` |
| TS-EMAIL-GRPC      | grpc      | `cargo test --package nvbes-email-worker email.worker.grpc`       |
| TS-EMAIL-RETENTION | retention | `cargo test --package nvbes-email-worker email.worker.retention`  |

Test cases `TC-EMAIL-<SUITE>-<NUM>` (template §4 de `03-documentation.md`). Le runner
keyword-driven couvre : `infra.setup_db`, `infra.migrate`, `infra.start_service`,
`infra.health_check`, `infra.cleanup` puis les mots-clés `email.send`,
`email.verify_delivery`, `email.capture_webhook`, `email.retry_failed` contre le
worker et le provider stub.

## Gates et acceptation

- PR : lint, check, unitaires système, contrats conteneur, migrations.
- Avant staging : `deployment-bootstrap` et `synthetic-smoke` verts.
- Avant production : snapshot operations sain (file d'attente, échecs, suppressions),
  aucun dead-letter non traité, approval explicite signée.
- Un bug de livraison commence par un test rouge : déduplication, rejeu, rétention ou
  acheminement au plus bas niveau pertinent.

## Principes de qualité

- At-least-once est le contrat : les tests de rejeu et dead-letter sont obligatoires.
- Dédoublement/suppression/rétention : les garanties se testent, elles ne se déclarent
  pas.
- Aucun envoi réel en PR/CI; provider stub uniquement.
- Pas de données réelles ni secrets de provider dans les artefacts.
- Le runtime email reste dans le plafond FinOps; un nouveau volume d'envois réels
  nécessite un calcul de coût démontré
