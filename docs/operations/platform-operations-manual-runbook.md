# Runbook Platform Operations — opérateur solo

## Objectif

Permettre à l'unique opérateur nvbes de traiter de façon sûre, cohérente et
auditable les demandes d'administration, support, sécurité, abus, facturation,
recours et modération pendant la V1.

Ce runbook applique la
[direction produit](../product/nvbes-product-strategy.md) et la
[conception Platform Operations](../superpowers/specs/2026-08-25-nvbes-v1-finops-platform-operations-design.md).

## Politique par défaut

- tout traitement est manuel tant qu'une automatisation conforme aux exigences
  et au budget global de 30 EUR TTC n'est pas démontrée ;
- Trust/Risk recommande en shadow mode, le domaine propriétaire décide ;
- aucune usurpation complète de session utilisateur ;
- aucune action sensible sans motif, preuve minimale et audit ;
- préférer une mesure temporaire, réversible et expirante ;
- ne jamais répéter aveuglément une commande au résultat incertain.

## Catégories de dossier

- compromission ou récupération de compte ;
- fraude, abus, bot ou signalement ;
- paiement, remboursement ou facturation ;
- délivrabilité et suppression Email ;
- demande relative aux données personnelles ;
- incident technique ;
- recours contre une décision opérateur.

La modération de contenu reste inactive tant qu'aucun produit hébergeant du
contenu n'est sélectionné. Un signalement reçu malgré tout est conservé comme
dossier d'abus et évalué selon son contexte réel.

## Traitement d'un dossier

1. créer un identifiant et classer la demande ;
2. enregistrer l'heure, la source et le service propriétaire ;
3. minimiser et préserver les preuves nécessaires ;
4. consulter les APIs et audits autorisés, jamais directement une base métier ;
5. qualifier urgence, impact, réversibilité et besoin d'escalade ;
6. choisir l'action la moins destructive qui protège la propriété concernée ;
7. exécuter une commande idempotente avec motif et expiration si applicable ;
8. enregistrer le résultat exact : succès, échec, `pending` ou `unknown` ;
9. notifier la personne concernée lorsque nécessaire ;
10. permettre le recours et clôturer seulement après vérification.

États : `open`, `investigating`, `awaiting_user`, `action_pending`, `resolved`,
`appealed`, `closed`.

## Actions sensibles

MFA et step-up sont obligatoires avant :

- suspension ou restriction d'un compte ;
- révocation globale de sessions ;
- modification d'un paiement ou remboursement ;
- accès exceptionnel à des données non masquées ;
- suppression irréversible ;
- changement d'un contrôle FinOps ou de sécurité.

Une suppression irréversible est différée, annulable pendant son délai de
sécurité et exige un nouveau step-up à l'exécution.

## Incidents et escalade

- utiliser les [runbooks incidents](incident-runbooks.md) pour la technique ;
- ouvrir la procédure de violation si des données personnelles sont concernées ;
- préserver les traces sans collecter de données supplémentaires inutiles ;
- ne pas promettre de SLA ou délai Enterprise ;
- documenter les limites lorsque l'opérateur ne peut pas conclure seul et
  obtenir l'avis légal, fiscal ou sécurité nécessaire avant une action risquée.

## Gate d'automatisation

Une tâche manuelle ne devient automatisée que si :

1. son volume, son temps ou son risque est mesuré ;
2. les règles d'entrée et de sortie sont déterministes ;
3. les erreurs, recours et kill switch sont prévus ;
4. l'automatisation est observable, testée, réversible et remplaçable ;
5. son coût complet maintient le projet sous 30 EUR TTC ;
6. elle ne prend pas seule une décision irréversible ou ambiguë.

Une solution maison est acceptable si la comparaison construire/acheter inclut
développement, maintenance, sécurité, panne et migration, et démontre un meilleur
coût total que les alternatives.

## Revue FinOps

Avant chaque mise en production et au minimum mensuellement :

1. collecter factures et estimations TTC de chaque fournisseur ;
2. les attribuer aux catégories du contrat budgétaire ;
3. vérifier la prévision à 30 jours ;
4. exécuter `pnpm check:finops` ;
5. appliquer les paliers 25/28/30 EUR si nécessaire ;
6. consigner toute réallocation, économie ou nouvelle charge.

Les alertes fournisseur sont informatives. Le contrôle manuel TTC reste
l'autorité jusqu'à livraison d'un rapprochement automatique fiable et gratuit.

## Ajout futur d'opérateurs

Le modèle prépare des permissions distinctes `support_agent`, `risk_reviewer`,
`billing_operator` et `security_admin`. Aucun nouveau rôle n'est attribué sans
moindre privilège, MFA, séparation lecture/action et audit. Le compte
`platform_owner` ne doit jamais être partagé.
