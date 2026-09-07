# Politique de Retention des Donnees

## Statut

Document de reference a tenir a jour avant publication.

Cette politique concerne les services Nvbes exploités par Rayane Guemmoud EI
(nom commercial OTAKIMI), identifié dans les
[mentions légales](./site-legal-notice.md). Elle complète la
[politique de confidentialité](./privacy-policy.md) pour les traitements Account.

## Principes

- Minimisation des donnees.
- Durees de conservation bornees.
- Suppression ou anonymisation quand la conservation n'est plus necessaire.
- Cohabitation explicite entre suppression active, corbeille, backups et obligations legales.
- Toute donnee persistante recoit une classification et une categorie de
  retention; la classification la plus restrictive prevaut.
- Une obligation de conservation ou un legal hold suspend la purge, jamais les
  controles d'acces ni le chiffrement.

## Classes techniques

| Classe         | Exemples                                   | Controle minimal                                    |
| -------------- | ------------------------------------------ | --------------------------------------------------- |
| `public`       | contenu explicitement publie               | integrite, sauvegarde                               |
| `internal`     | metadonnees de service sans contenu client | acces employe limite, journalisation                |
| `confidential` | fichiers et donnees metier client          | chiffrement par enveloppe, isolation tenant         |
| `restricted`   | secrets, facteurs MFA, preuves de securite | cle dediee, acces step-up, audit, purge prioritaire |

Le registre executable `data_retention_policies` constitue la source de verite
pour les workers. Les valeurs de ce document sont les bornes produit et
reglementaires; une configuration de plan ne peut que raccourcir une duree,
sauf obligation legale explicite.

## Regles cibles V1

| Catégorie                                         | Durée de Conservation                                                                             | Justification                                                    |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Données de compte actif                           | Pendant la durée du contrat                                                                       | Exécution du contrat                                             |
| Sessions actives                                  | Jusqu'à expiration ou révocation (max 30 jours)                                                   | Sécurité de l'accès                                              |
| Logs d'authentification et sécurité               | 1 an                                                                                              | Sécurité et détection d'intrusions (ANSSI/CNIL)                  |
| Audit logs produit (actions admin)                | 1 an (Plan Pro) / 7 ans (Plan Enterprise)                                                         | Preuve et conformité                                             |
| Documents de facturation                          | 10 ans                                                                                            | Obligation comptable et fiscale (L123-22 Code Com)               |
| Tickets support et échanges                       | 5 ans après clôture                                                                               | Preuve contractuelle et défense juridique                        |
| Métriques observabilité                           | 13 mois maximum                                                                                   | Pilotage technique et tendances de fiabilité                     |
| Logs techniques redigés                           | 30 à 90 jours                                                                                     | Diagnostic incident et sécurité opérationnelle                   |
| Traces distribuées redigées                       | 7 à 30 jours                                                                                      | Diagnostic performance et erreurs                                |
| Profils CPU continus                              | 7 à 30 jours                                                                                      | Optimisation performance sans contenu utilisateur                |
| Charge utile des communications transactionnelles | 30 jours après un statut terminal ; borne indépendante du webhook à implémenter avant publication | Exécution du message puis minimisation                           |
| Registre de cycle de vie et événements de remise  | 400 jours après statut terminal ou traitement de l’événement                                      | Preuve de remise, diagnostic et gestion des incidents            |
| Liste de suppression email                        | Tant que le motif de suppression demeure ; durée maximale et réexamen à finaliser                 | Prévenir les rebonds répétés, le spam et les envois indésirables |
| Metadata Scaleway Generative APIs                 | Jusqu'à 6 mois, sous forme agregee ou anonymisee                                                  | Performance, fiabilite et amelioration du service d'inference    |
| Contenu de requete Scaleway Generative APIs       | Jusqu'à 2 semaines uniquement en cas d'incident, abus, erreur anormale ou investigation securite  | Reproduction, investigation et correction d'incident             |
| Workspaces résiliés / impayés                     | 30 jours (Accès lecture) + 7 jours (Purge technique)                                              | Récupération des données et minimisation                         |
| Fichiers en corbeille                             | 30 jours (par défaut)                                                                             | Droit à l'erreur et minimisation                                 |
| Données supprimées (Tombstones)                   | 30 jours avant purge physique                                                                     | Cohérence des backups et intégrité technique                     |
| Backups                                           | 30 jours (Rotation glissante)                                                                     | Plan de continuité d'activité                                    |

## 3. Modalités Techniques de Suppression

- **Suppression Logique (Soft Delete)**: La donnée est marquée comme supprimée et n'est plus accessible via les interfaces standard. Elle reste présente en base pour assurer l'intégrité référentielle et permettre une restauration rapide en cas d'erreur.
- **Purge Physique (Hard Delete)**: Un worker de maintenance (`cloud-worker-maintenance`) parcourt périodiquement les données marquées pour suppression dont le délai de rétention est expiré et procède à leur destruction irréversible sur le stockage objet (Scaleway S3).
- **Suppression cryptographique**: pour les categories qui l'exigent, la cle de
  donnees unique de l'objet est detruite avant la purge physique. La suppression
  est horodatee dans `cryptographic_erased_at`; aucun ciphertext ne doit etre
  considere comme efface tant que son enveloppe de cle existe encore.
- **Backups**: Les données supprimées physiquement disparaissent des sauvegardes au fur et à mesure de la rotation des cycles de backup (30 jours).
- **Observabilité Grafana Cloud**: les signaux quittent l'application via
  Grafana Alloy uniquement. Alloy applique redaction, sampling, labels
  techniques et routage avant export. Les durées effectives doivent être
  configurées dans Grafana Cloud pour rester dans les bornes ci-dessus.
- **Scaleway Generative APIs**: les prompts, sorties et contenus transmis aux
  fonctionnalites IA ne sont pas conserves par defaut par le sous-traitant. Une
  conservation temporaire du contenu complet de requete peut avoir lieu cote
  Scaleway uniquement pour diagnostiquer une erreur anormale, un abus, une
  degradation du service ou une investigation securite.
- **Scaleway TEM**: le prestataire annonce supprimer automatiquement le contenu
  du message apres traitement. Les durees de ses metadonnees d'activite et de
  ses listes de blocage, ainsi que la procedure de deblocage ou d'effacement,
  doivent etre confirmees contractuellement avant publication.

## 4. Exercice des Droits

En cas de demande d'effacement (Article 17 du RGPD), les données sont supprimées sans délai injustifié, sous réserve des obligations légales de conservation (notamment facturation et sécurité).

## 5. Revision de la Politique

Cette politique est revue annuellement ou lors de changements majeurs dans l'architecture technique du service.
