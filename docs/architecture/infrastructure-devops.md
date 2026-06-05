# Infrastructure, Network et DevOps

## Objectif

Ce document definit les exigences minimales d'exploitation pour nvbes Drive V1.

Il couvre le reseau, les environnements, CI/CD, migrations, backups, observabilite, workers, object storage, incident response et runbooks.

## Topologie Reseau V1

Principe:

- Exposer uniquement ce qui doit etre public.
- Garder les bases de donnees, workers et outils internes sur reseau prive.
- Faire passer le trafic public par Cloudflare.

Composants publics:

- Frontend web via Cloudflare/CDN.
- API publique via Cloudflare.
- Routes publiques de partage via Cloudflare avec rate limiting dedie.
- Signed URLs Object Storage limitees, courtes et scopees.

Composants prives:

- PostgreSQL.
- Workers.
- Interfaces admin internes.
- Jobs et consoles d'exploitation.
- Monitoring interne si expose via dashboard.

Regles reseau:

- PostgreSQL n'est jamais expose publiquement.
- L'API accede a PostgreSQL via reseau prive ou allowlist stricte.
- Les workers accedent a PostgreSQL via reseau prive ou allowlist stricte.
- Les buckets Object Storage restent prives.
- Aucun bucket public en production.
- Les flux sortants vers providers externes sont documentes: billing, email, monitoring, anti-malware.
- Les security groups/firewalls autorisent seulement les ports necessaires.
- L'acces admin passe par Cloudflare Zero Trust ou mecanisme equivalent.
- Les IP d'administration sont limitees quand le fournisseur le permet.

## Environnements

Environnements minimaux:

- `development`
- `staging`
- `production`

Chaque environnement a:

- Base PostgreSQL dediee.
- Redis dedie pour les sessions, refresh tokens et autres etats techniques hot-path.
- Buckets Object Storage dedies.
- Secrets dedies.
- Webhooks billing dedies.
- Domaine ou sous-domaine dedie.
- Logs separes.
- Configuration Cloudflare separee ou clairement isolee.
- Comptes de service separes quand possible.

Regles:

- Aucune donnee production en development.
- Les donnees production utilisees en debug doivent etre anonymisees ou synthetiques.
- Les secrets production ne sont jamais accessibles aux environnements non-production.
- Les migrations passent d'abord en staging.
- Les tests d'integration critiques tournent en staging avant promotion production.

## Infrastructure as Code

Toute infrastructure critique doit etre decrite en IaC avant production.

Perimetre IaC V1:

- DNS et regles Cloudflare critiques.
- Buckets Object Storage.
- PostgreSQL.
- Compute API.
- Compute workers.
- Security groups/firewalls.
- Variables d'environnement non secretes.
- Policies bucket.
- Backups et retention.
- Monitoring/alerting critique si le provider le permet.

Regles:

- Les changements infra passent par review.
- Les changements production sont tracables.
- Les credentials IaC sont separes par environnement.
- Aucun changement manuel production durable sans backport dans IaC.

## CI/CD

Pipeline minimal:

1. Lint et format.
2. Tests unitaires.
3. Tests d'integration critiques.
4. Build frontend.
5. Build backend.
6. Scan dependances.
7. Scan secrets.
8. Build artefacts immutables.
9. Migration check.
10. Deploiement staging.
11. Smoke tests staging.
12. Promotion production avec approval.
13. Smoke tests production.

La strategie globale de test et la matrice de regression sont definies dans:

- [Strategie de test](../testing/test-strategy.md)
- [Matrice de regression](../testing/regression-matrix.md)

Contraintes:

- Les artefacts deployes en production sont ceux valides en staging.
- Les secrets sont injectes par environnement, jamais construits dans l'artefact.
- Le rollback doit etre documente avant la premiere mise en production.
- Chaque deploiement production genere un changelog operationnel.
- Les migrations destructives necessitent approval explicite.
- La suite de regression critique doit etre verte avant promotion production.
- Les smoke tests ne remplacent pas les tests de regression.

## Migrations PostgreSQL

Regles:

- Migrations versionnees.
- Migrations appliquees automatiquement par pipeline ou job controle.
- Migration check obligatoire en CI.
- Les migrations doivent etre compatibles zero-downtime quand possible.
- Les changements schema suivent un modele expand/contract.
- Les migrations longues sont executees en batch ou job dedie.
- Rollback documente pour chaque migration risquee.
- Backup ou snapshot avant migration production sensible.
- L'API doit rester compatible avec l'ancien et le nouveau schema pendant la promotion.

## Backups et Restore

Objectifs V1:

- RPO PostgreSQL cible: 24h maximum au lancement.
- RTO PostgreSQL cible: 4h maximum au lancement.
- RPO Object Storage: selon versioning/lifecycle du bucket.
- RTO Object Storage: procedure documentee par type d'incident.

Exigences:

- Backups PostgreSQL automatiques et chiffres.
- Retention backup par environnement documentee.
- Backups production separes des environnements non-production.
- Tests de restauration planifies au minimum mensuellement avant maturite SOC 2.
- Test de restore obligatoire avant lancement public.
- Validation post-restore: integrite DB, presence fichiers, quotas coherents, auth fonctionnelle.
- Procedure de restauration partielle documentee pour workspace ou objet critique.
- Les restaurations doivent rejouer ou respecter les demandes de suppression RGPD deja traitees.

## Object Storage

Regles V1:

- Buckets prives.
- Policies bucket minimales.
- Pas de listing public.
- Signed URLs courtes.
- CORS limite aux domaines autorises.
- Taille maximale par fichier definie par plan.
- Multipart upload supporte si necessaire pour gros fichiers.
- Nettoyage automatique des multipart uploads incomplets.
- Lifecycle rules pour fichiers `pending`, corbeille et suppression definitive.
- Versioning bucket a evaluer avant production selon cout et besoin de restore.
- Replication ou strategie multi-region a evaluer apres validation du modele economique.
- Verification reguliere qu'aucune policy publique n'est activee en production.

## Observabilite

Objectif V1:

- Donner une visibilite operationnelle suffisante avant ouverture publique.
- Correlier chaque incident avec un `request_id`.
- Ne jamais utiliser le meme flux pour debug technique, preuve audit et analytics produit.
- Rester portable tant que le provider final d'observabilite n'est pas fige.

Logs:

- Logs structures JSON.
- `request_id` ou `correlation_id` sur chaque requete.
- Header entrant et sortant: `x-request-id`.
- Champ obligatoire des logs techniques: `event_category=technical_log`.
- Redaction des secrets, tokens, signed URLs et donnees sensibles.
- Retention des logs definie par environnement.
- Audit logs separes des logs applicatifs classiques.
- Analytics produit separees des logs techniques et des audit events.

Flux separes:

| Flux            | Finalite                                        | Donnees autorisees                                                                 | Donnees interdites                                  |
| --------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------- |
| Logs techniques | Exploitation, debug, performance, incidents     | `request_id`, methode, path template, statut, duree, service, environnement        | tokens, cles API, signed URLs, contenu fichier      |
| Audit           | Preuve produit/securite consultable Owner/Admin | workspace, acteur, action, cible, IP si necessaire, `request_id`, metadata limitee | secret de cle API, mot de passe, contenu fichier    |
| Analytics       | Mesure activation, conversion, retention        | workspace pseudonymise, plan, event name, cohort, source/campaign consentie        | emails, noms fichiers, object keys, tokens, contenu |

Implementation API V1:

- Middleware HTTP transversal pour generer ou reprendre `x-request-id`.
- Propagation de `x-request-id` dans la reponse.
- Logs techniques `http.request` avec statut et duree.
- Endpoint `/metrics` pour compteurs HTTP minimaux en memoire.
- Endpoint `/observability/dashboards` pour definitions minimales de dashboards.
- Endpoint `/observability/alerts/critical` pour definitions d'alertes critiques.
- Endpoint `/observability/log-streams` pour documenter la separation des flux.

Metriques:

- Latence API.
- Taux d'erreurs 4xx/5xx.
- Upload success/failure.
- Download success/failure.
- Taille et duree des uploads.
- Queue depth des workers.
- Jobs failed/retried/dead-letter.
- Connexions PostgreSQL.
- Latence PostgreSQL.
- Erreurs Object Storage.
- Webhooks billing recus/rejetes/echoues.
- Usage stockage par workspace.
- Cout estime par workspace.
- Cout estime par plan.
- Egress par workspace.
- Operations Object Storage par workspace.
- Volume logs par environnement.

Alertes critiques:

- API 5xx eleve.
- Upload failures eleves.
- Download failures eleves.
- Workers bloques ou queue en croissance.
- Jobs RGPD ou billing en echec.
- PostgreSQL indisponible ou proche saturation.
- Object Storage inaccessible.
- Backup echoue.
- Certificat/TLS ou domaine en erreur.
- Suspicion de bucket public.
- Budget cloud mensuel a 80%.
- Budget cloud mensuel a 100%.
- Workspace avec marge brute negative.
- Trial avec cout anormal.
- Egress anormal par workspace.

Dashboards V1:

- API health.
- Upload/download health.
- Workers/jobs.
- PostgreSQL.
- Object Storage.
- Billing webhooks.
- Security events.
- FinOps: couts par environnement, plan et workspace.
- Product analytics: activation, conversion, retention et usage coeur.

Dashboards minimum avant lancement:

| Dashboard       | Panneaux minimum                                                                |
| --------------- | ------------------------------------------------------------------------------- |
| API health      | volume par statut, latence p95, taux 5xx, requetes actives                      |
| Upload/download | succes/echecs upload, succes/echecs download, duree upload, erreurs storage     |
| Workers/jobs    | queue depth, jobs failed, jobs retried, dead letters                            |
| Security events | auth failures, permission denied, api key denied, audit events crees            |
| FinOps          | stockage par workspace, egress par workspace, volume logs, marge brute par plan |

Alertes critiques minimum:

| Alerte                      | Condition V1                                                         | Owner       |
| --------------------------- | -------------------------------------------------------------------- | ----------- |
| API 5xx eleve               | Taux 5xx superieur a 2% pendant 5 minutes                            | Engineering |
| Upload failures eleves      | Taux d'echec upload superieur a 5% pendant 10 minutes                | Engineering |
| Worker queue bloquee        | Queue en croissance 15 minutes ou plus vieux job au-dessus de 30 min | Engineering |
| PostgreSQL saturation       | DB indisponible ou pool au-dessus de 90%                             | Engineering |
| Object Storage indisponible | Erreurs storage au-dessus du seuil pendant 5 minutes                 | Engineering |
| Budget cloud 100%           | Budget mensuel atteint                                               | Ops/Finance |
| Bucket public suspect       | Scan policy detecte un acces public                                  | Security    |

## Observabilite Produit

Exigences:

- Event taxonomy documentee avant implementation.
- Separation stricte entre product analytics, audit logs et logs techniques.
- Analytics au niveau workspace quand possible.
- Pas de donnees sensibles dans les events analytics.
- Dashboards activation, conversion, retention, revenue et FinOps.
- Attribution minimale source/campaign sans fingerprinting invasif.
- Consentement analytics marketing gere avant tracking non essentiel.
- Le funnel V1 ne doit pas imposer un choix de plan avant la premiere activation produit.

## FinOps Infrastructure

Exigences:

- Budgets cloud definis par environnement.
- Tags ou labels de cout sur ressources critiques quand le provider le permet.
- Cout compute API suivi.
- Cout workers suivi.
- Cout PostgreSQL suivi.
- Cout Object Storage suivi.
- Cout egress suivi.
- Cout backups suivi.
- Cout logs/monitoring suivi.
- Cout anti-malware suivi.
- Revue mensuelle des couts avant maturite.
- Alerte sur toute derive de cout variable liee aux trials.

## Workers et Jobs

Les jobs V1 sont portes par une queue dediee, pas par des tables PostgreSQL de polling:

- Jobs idempotents.
- Retries limites.
- Backoff exponentiel.
- Timeout par type de job.
- Locking pour eviter double execution concurrente.
- Statuts clairs: pending, running, succeeded, failed, dead-letter.
- Dead-letter queue dediee.
- Reprise apres crash.
- Priorites pour jobs critiques: billing, RGPD, quotas.
- Monitoring queue depth et taux d'echec.
- Alertes sur jobs critiques en echec.
- Journalisation des tentatives et erreurs.

## Incident Response Operationnel

Severites:

- SEV1: indisponibilite majeure, fuite de donnees, bucket public, corruption ou perte de donnees.
- SEV2: degradation importante, uploads/downloads massivement affectes, billing incorrect.
- SEV3: degradation partielle ou bug sans impact donnees critique.

Exigences:

- Canal d'alerte interne defini.
- Owner d'incident designe.
- Runbooks pour API down, DB down, Object Storage down, bucket public, billing webhook failure, job RGPD failure.
- Criteres de rollback documentes.
- Communication client preparee pour incidents SEV1/SEV2.
- Preservation des logs pendant incident.
- Postmortem obligatoire pour SEV1/SEV2.
- Exercices d'incident planifies avant maturite SOC 2.
