# Annexes à l’Accord relatif au traitement de données à caractère personnel

**Version rattachée à l’Accord : 2026-07-30**

Les présentes annexes font partie intégrante de l’Accord de même version.

# Annexe I — Description du traitement

## A. Parties, contacts et qualification

| Élément | Client | nvbes |
| --- | --- | --- |
| Identité complète | `[À COMPLÉTER]` | nvbes Cloud SAS — mentions à compléter |
| Rôle | Responsable du traitement | Sous-traitant |
| Contact protection des données | `[À COMPLÉTER]` | privacy@nvbes.cloud |
| DPO | `[À COMPLÉTER]` | `[À COMPLÉTER OU NON DÉSIGNÉ]` |
| Contact incident disponible en permanence | `[À COMPLÉTER]` | `[À COMPLÉTER]` |

## B. Traitements confiés

| Élément requis | Description contractuelle |
| --- | --- |
| Objet | Fourniture au Client de capacités professionnelles d’identité, d’administration de comptes et d’authentification au moyen de nvbes Account |
| Durée | Durée du Contrat principal, augmentée des seuls délais de restitution, suppression ou conservation légale prévus par l’Accord |
| Nature et opérations | Collecte, enregistrement, organisation, structuration, consultation, authentification, transmission autorisée, journalisation, hébergement, sauvegarde, export, restriction et suppression |
| Finalités | Créer et administrer les identités désignées par le Client ; authentifier les utilisateurs ; appliquer les politiques d’accès configurées ; maintenir la sécurité ; exécuter les autorisations et fournir l’assistance convenue |
| Fréquence | Continue pendant l’utilisation du Service |
| Personnes concernées | Utilisateurs finaux, administrateurs et personnes invitées relevant du périmètre du Client |
| Données d’identification | Identifiant, nom, prénom, nom d’utilisateur, adresse électronique, région et attributs de profil configurés |
| Données d’authentification | Empreinte de mot de passe, clés publiques WebAuthn, facteurs MFA protégés, codes de récupération hachés et événements d’authentification |
| Données techniques et de sécurité | IP, agent utilisateur, appareil, session, journaux, signaux et scores de risque |
| Autorisations | Applications, permissions, portées, attributs transmis et historique de révocation |
| Catégories particulières, article 9 | Aucune catégorie particulière n’est destinée à être traitée. Tout traitement exceptionnel doit être préalablement décrit, autorisé et assorti de garanties renforcées |
| Données d’infraction, article 10 | Aucune, sauf avenant identifiant le fondement et les garanties |
| Localisation principale | `[PAYS ET RÉGIONS À COMPLÉTER]` |
| Format de restitution | JSON structuré et tout format supplémentaire convenu : `[À COMPLÉTER]` |

## C. Traitements autonomes de nvbes exclus de l’Accord

| Finalité propre | Données | Base juridique | Durée |
| --- | --- | --- | --- |
| `[À COMPLÉTER OU INDIQUER « AUCUN »]` | `[À COMPLÉTER]` | `[À COMPLÉTER]` | `[À COMPLÉTER]` |

## D. Instructions particulières

- Politiques de conservation propres au Client : `[À COMPLÉTER]`.
- Restrictions géographiques : `[À COMPLÉTER]`.
- Personnes habilitées à émettre des instructions : `[À COMPLÉTER]`.
- Contraintes relatives aux catégories particulières : `[À COMPLÉTER]`.
- Autres instructions : `[À COMPLÉTER]`.

# Annexe II — Mesures techniques et organisationnelles

À la Date d’effet, nvbes met en œuvre et maintient au minimum les mesures
suivantes.

Les paramètres ci-dessous sont complétés avec la baseline réellement déployée
avant signature. Une mention générique ne vaut pas description contractuelle :

| Paramètre de sécurité | Baseline contractuelle |
| --- | --- |
| Chiffrement en transit | `[PROTOCOLES, VERSIONS MINIMALES ET PÉRIMÈTRE]` |
| Chiffrement au repos et gestion des clés | `[ALGORITHMES, PÉRIMÈTRE, KMS, ROTATION ET SÉPARATION]` |
| Révision des accès privilégiés | `[FRÉQUENCE ET RESPONSABLE]` |
| Conservation des journaux de sécurité | `[DURÉE ET PROTECTION CONTRE L’ALTÉRATION]` |
| Analyse de vulnérabilités et tests d’intrusion | `[FRÉQUENCE ET PÉRIMÈTRE]` |
| Remédiation des vulnérabilités | `[DÉLAIS CRITIQUE, ÉLEVÉ, MOYEN ET FAIBLE]` |
| Sauvegardes | `[FRÉQUENCE, CHIFFREMENT, DURÉE ET TEST DE RESTAURATION]` |
| Continuité | `[RPO, RTO ET FRÉQUENCE D’EXERCICE]` |
| Suppression active et sauvegardes | Trente jours au plus pour les données actives et la rotation des sauvegardes |
| Certifications et rapports indépendants | `[À COMPLÉTER OU INDIQUER « AUCUN »]` |

## 1. Gouvernance et personnel

- politiques de sécurité et de protection des données approuvées et révisées ;
- engagement de confidentialité, formation périodique et gestion des
  habilitations ;
- contrôle d’antécédents lorsque la loi l’autorise et que le risque le justifie.

## 2. Contrôle des accès

- identification individuelle, moindre privilège et séparation des fonctions ;
- authentification multifacteur pour les accès administratifs ;
- révision périodique des habilitations et journalisation des accès privilégiés.

## 3. Protection cryptographique

- chiffrement en transit et au repos selon l’état de l’art ;
- hachage non réversible et salé des mots de passe et codes de récupération ;
- chiffrement des secrets MFA devant pouvoir être restitués au Service ;
- gestion, rotation, séparation et révocation documentées des clés.

## 4. Isolation et minimisation

- séparation logique des périmètres clients et des environnements ;
- interdiction des Données du Client en test sans mesure approuvée ;
- minimisation, pseudonymisation lorsque possible et exclusion des secrets des
  journaux.

## 5. Journalisation et surveillance

- traçabilité des accès, modifications, authentifications et actions sensibles ;
- protection, durée documentée, alertes et détection d’anomalies ;
- investigation et préservation des preuves.

## 6. Développement et vulnérabilités

- revues de code et contrôles automatisés proportionnés au risque ;
- gestion des dépendances, vulnérabilités et correctifs selon leur criticité ;
- tests de sécurité réguliers et séparation des secrets.

## 7. Disponibilité et continuité

- sauvegardes chiffrées, durée maximale et tests de restauration ;
- redondance, reprise, continuité et gestion de crise proportionnées au Service.

## 8. Incidents

- alerte, qualification, confinement, éradication et restauration ;
- notification selon l’article 9, analyse de cause et mesures correctives ;
- registre des Violations et conservation des preuves.

## 9. Suppression et restitution

- export structuré, suppression contrôlée des données actives et réplicas ;
- expiration des jetons et exports, rotation des sauvegardes et attestation de
  suppression.

## 10. Sous-traitance

- évaluation préalable, mêmes obligations contractuelles et suivi des
  garanties, Transferts, incidents et audits.

# Annexe III — Sous-traitants ultérieurs autorisés

L’annexe doit être entièrement complétée avant la Date d’effet. Une catégorie
générique de prestataires ne constitue pas une liste convenue.

| Dénomination et adresse | Pays de traitement et d’accès | Prestation | Données concernées | Mécanisme de Transfert |
| --- | --- | --- | --- | --- |
| `[À COMPLÉTER]` | `[À COMPLÉTER]` | Hébergement et base de données | `[À COMPLÉTER]` | `[À COMPLÉTER OU SANS OBJET]` |
| `[À COMPLÉTER]` | `[À COMPLÉTER]` | Réseau et sécurité | `[À COMPLÉTER]` | `[À COMPLÉTER OU SANS OBJET]` |
| `[À COMPLÉTER]` | `[À COMPLÉTER]` | Communications transactionnelles | `[À COMPLÉTER]` | `[À COMPLÉTER OU SANS OBJET]` |
| `[À COMPLÉTER]` | `[À COMPLÉTER]` | Observabilité et gestion d’incident | `[À COMPLÉTER]` | `[À COMPLÉTER OU SANS OBJET]` |

# Annexe IV — Registre des transferts internationaux

Aucun flux ne peut commencer avant la complétion de la présente annexe et,
lorsque des clauses contractuelles types sont nécessaires, la conclusion
séparée du texte non modifié de la décision d’exécution (UE) 2021/914 avec ses
options et annexes. Le présent tableau ne constitue pas ces clauses.

| Flux et données | Exportateur, rôle et pays | Importateur, rôle et pays | Garantie, référence et date | Module et options 2021/914 | Référence de l’évaluation et mesures supplémentaires |
| --- | --- | --- | --- | --- | --- |
| `[À COMPLÉTER OU « AUCUN »]` | `[À COMPLÉTER]` | `[À COMPLÉTER]` | `[ADÉQUATION, CCT OU AUTRE GARANTIE]` | `[MODULE ET OPTIONS, OU SANS OBJET]` | `[À COMPLÉTER OU SANS OBJET]` |
