# Stratégie de test Trust/Risk

> **Statut : socle V1 actif.** Le trust-risk-service est indépendant d'Identity :
> ingestion de signaux, projection de features, évaluation de risque (score/bande/
> recommandation), labels, review cases et cycle de vie des règles. Le mode par défaut
> est le shadow mode : les décisions ne se substituent pas au jugement opérateur.

## Portée et statut de preuve

Cette stratégie couvre `apps/trust-risk-service` et `nvbes-trust-risk` (types de
signaux, évaluation, moteur de règles, feature map). L'interface est gRPC ;
le web HTTP ne sert que les sondes et métriques.

Interfaces réelles :

| Protocole/Service                            | Objet                                                                                |
| -------------------------------------------- | ------------------------------------------------------------------------------------ |
| HTTP `GET /health/live`                      | liveness                                                                             |
| HTTP `GET /health/ready`                     | readiness (DB + heartbeat projection)                                                |
| HTTP `GET /metrics`                          | Prometheus (Bearer) + OTLP Grafana                                                   |
| gRPC `TrustRiskSignalService/SubmitSignals`  | ingestion de signaux                                                                 |
| gRPC `TrustRiskAssessmentService/AssessRisk` | évaluation de risque                                                                 |
| gRPC `TrustRiskLabelService/SubmitLabels`    | labels                                                                               |
| gRPC `TrustRiskOperationsService`            | GetEvaluation, ListReviewCases, TransitionReviewCase, Stage/Activate/RollbackRuleSet |

Projection : worker 250 ms, dérivation de features depuis les signaux, watermark;
worker de rétention.

## État d'automatisation réel

| Lane             | Déclenchement réel                    | Couverture actuelle                                                                                                  | Preuve             |
| ---------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------ |
| PR               | CI du dépôt                           | unitaires : health, config, auth, projection, operations, retention, acceptance (inline main.rs) + 5 blocs db inline | rapports Cargo     |
| Database         | lane `database` CI (PostgreSQL isolé) | ingress, assessment, rules (feature `database-tests`)                                                                | `test:database` Nx |
| Containers       | lane `containers` CI                  | contrat conteneur : reproduisibilité, non-root, migrations avant serveur                                             | `test:contract` Nx |
| Trusted hermetic | commande opérateur                    | PostgreSQL éphémère + service isolé, projection observée, scenarii d'évaluation                                      | journaux signés    |
| Pre-release      | session signée                        | campagnes de review cases, règles stagées/activées puis rollback démontré                                            | rapports signés    |

### Limites explicites

- L'évaluation n'a pas de référence d'or indépendante V1 (pas de dataset labellisé
  externe) : l'acceptance repose sur des scenarii synthétiques et des properties
  (score/bande/recommandation cohérents avec la feature map).
- Le service ne bloque aucun flux V1 : toute règle est shadow et observée dans des
  review cases. Le passage en mode pilotage/forcement est un gap conditionnel soumis
  à démonstration.
- Le `erase-subject` (RGPD) est testé en unitaire/DB; il n'a pas encore de campagne
  de compliance spécifique (à activer si le volume de demandes le justifie).
- Pas de campagne UI : pas de frontend branché V1.

## Fiabilité et sécurité du harness

- La projection (dérivation de features, watermark) a des tests dédiés; la watermark
  est la garantie anti-rejeu de signaux déjà projetés.
- L'ingestion est testée contre livermode/rejeu : rien dans le runtime test ne flague
  un doublon de signaux comme vert sans preuve.
- Le cycle de vie des règles (Stage → Activate → Rollback) est testé avec plombier
  réel : un rollback démontré appartient à la suite DB.
- Les review cases sont tracés (actor + raison) et testés; aucun transition invalide
  ne passe.
- Les artefacts sont rédigés : pas de données personnelles réelles ni de signaux
  sensibles dans les traces.

## Environnement et données de test

- PostgreSQL éphémère migré depuis zéro (1 migration) pour la lane database.
- Commandes CLI : `migrate`, `validate-runtime`, `error-reporting-smoke`,
  `erase-subject <kind> <namespace> <opaque-id> <actor> <reason>`.
- Budget : le service Trust/Risk est couvert par le plafond global de 30 EUR TTC/mois
  (métriques Prometheus + OTLP sous contrôle).

## Suites ISO 29119-3

Suites du §3.2 de `03-documentation.md` :

| Suite ID                | Catégorie | Commande réelle                                                       |
| ----------------------- | --------- | --------------------------------------------------------------------- |
| TS-TRUST-RISK-INGESTION | trust     | `cargo test --package nvbes-trust-risk-service trust_risk.ingress`    |
| TS-TRUST-RISK-ASSESS    | trust     | `cargo test --package nvbes-trust-risk-service trust_risk.assessment` |
| TS-TRUST-RISK-RULES     | trust     | `cargo test --package nvbes-trust-risk-service trust_risk.rules`      |

Test cases `TC-TRUST-RISK-<SUITE>-<NUM>` (template §4 de `03-documentation.md`). Le runner
keyword-driven couvre l'infrastructure (`infra.setup_db`, `infra.migrate`,
`infra.start_service`, `infra.health_check`, `infra.cleanup`). Le service Trust/Risk
n'expose **aucune route HTTP métier** : sa preuve de comportement passe par les tests Rust
gRPC (`TrustRiskClient` dans `libs/rust/trust-risk`) et, au niveau du runner keyword-driven,
par l'adaptateur JSON→proto des mots-clés `trust.*` (`trust.submit_signals`,
`trust.assess_risk`, `trust.submit_labels` — voir §2.4 de `05-keyword-driven.md`). Aucun
endpoint HTTP fictif n'est introduit.

## Gates et acceptation

- PR : lint, check, unitaires, contrats conteneur, migrations.
- Avant staging : projection saine (watermark à jour sur les scenarii de test).
- Avant production : aucune review case critique non résolue, règles stagées puis
  activées avec démonstration de rollback, approval explicite signée.
- Un bug de modèle (score, bande, règle, feature) commence par un test rouge au plus
  bas niveau qui le reproduit.

## Principes de qualité

- Shadow by default : aucune règle ne verrouille un flux sans démonstration.
- Les règles se testent par le cycle de vie complet (stage/activate/rollback).
- Pas de gap marké vert : une catégorie sans harness réel est un gap déclaré.
- Privacy-first : erase-subject reste fonctionnel et testé.
- Le service respecte le plafond FinOps; les métriques OTLP
