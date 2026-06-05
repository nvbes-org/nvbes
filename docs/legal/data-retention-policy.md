# Politique de Retention des Donnees

## Statut

Document de reference a tenir a jour avant publication.

## Principes

- Minimisation des donnees.
- Durees de conservation bornees.
- Suppression ou anonymisation quand la conservation n'est plus necessaire.
- Cohabitation explicite entre suppression active, corbeille, backups et obligations legales.

## Regles cibles V1

| Catégorie                           | Durée de Conservation                                   | Justification                                      |
| ----------------------------------- | ------------------------------------------------------- | -------------------------------------------------- |
| Données de compte actif             | Pendant la durée du contrat                             | Exécution du contrat                               |
| Sessions actives                    | Jusqu'à expiration ou révocation (max 30 jours)         | Sécurité de l'accès                                |
| Logs d'authentification et sécurité | 1 an                                                    | Sécurité et détection d'intrusions (ANSSI/CNIL)    |
| Audit logs produit (actions admin)  | 1 an (Plan Pro) / 7 ans (Plan Enterprise)               | Preuve et conformité                               |
| Documents de facturation            | 10 ans                                                  | Obligation comptable et fiscale (L123-22 Code Com) |
| Tickets support et échanges         | 5 ans après clôture                                     | Preuve contractuelle et défense juridique          |
| Métriques observabilité             | 13 mois maximum                                         | Pilotage technique et tendances de fiabilité       |
| Logs techniques redigés             | 30 à 90 jours                                           | Diagnostic incident et sécurité opérationnelle     |
| Traces distribuées redigées         | 7 à 30 jours                                            | Diagnostic performance et erreurs                  |
| Profils CPU continus                | 7 à 30 jours                                            | Optimisation performance sans contenu utilisateur  |
| Workspaces résiliés / impayés       | 30 jours (Accès lecture) + 7 jours (Purge technique)    | Récupération des données et minimisation           |
| Fichiers en corbeille               | 30 jours (par défaut)                                   | Droit à l'erreur et minimisation                   |
| Données supprimées (Tombstones)     | 30 jours avant purge physique                           | Cohérence des backups et intégrité technique       |
| Backups                             | 30 jours (Rotation glissante)                           | Plan de continuité d'activité                      |

## 3. Modalités Techniques de Suppression

- **Suppression Logique (Soft Delete)**: La donnée est marquée comme supprimée et n'est plus accessible via les interfaces standard. Elle reste présente en base pour assurer l'intégrité référentielle et permettre une restauration rapide en cas d'erreur.
- **Purge Physique (Hard Delete)**: Un worker de maintenance (`drive-worker-maintenance`) parcourt périodiquement les données marquées pour suppression dont le délai de rétention est expiré et procède à leur destruction irréversible sur le stockage objet (Scaleway S3).
- **Backups**: Les données supprimées physiquement disparaissent des sauvegardes au fur et à mesure de la rotation des cycles de backup (30 jours).
- **Observabilité Grafana Cloud**: les signaux quittent l'application via
  Grafana Alloy uniquement. Alloy applique redaction, sampling, labels
  techniques et routage avant export. Les durées effectives doivent être
  configurées dans Grafana Cloud pour rester dans les bornes ci-dessus.

## 4. Exercice des Droits

En cas de demande d'effacement (Article 17 du RGPD), les données sont supprimées sans délai injustifié, sous réserve des obligations légales de conservation (notamment facturation et sécurité).

## 5. Revision de la Politique

Cette politique est revue annuellement ou lors de changements majeurs dans l'architecture technique du service.
