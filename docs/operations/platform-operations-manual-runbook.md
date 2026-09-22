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

L'[API opérateur et son client manuel](platform-operations-api.md) implémentent
le registre de dossiers, les observations par domaine, les transitions et
l'audit transactionnel. Ce guide décrit la configuration et les commandes
exécutables, ainsi que les actions métier volontairement indisponibles.

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

## Procédures opérateur Identity V1

### Rotation de la clé de chiffrement MFA (AES-256-GCM)

1. Générer une nouvelle clé de 32 octets encodée en base64.
2. Déployer avec `NVBES_IDENTITY_MFA_KEY_VERSION` incrémenté, la nouvelle clé
   active, et conserver l'ancienne clé dans la variable
   `NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY` avec
   `NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION`.
3. Exécuter la commande de maintenance `rotate-mfa-key`.
4. Vérifier que le nombre de facteurs migrés correspond aux facteurs actifs de
   l'ancienne version.
5. Retirer la clé précédente lors du déploiement suivant.

### Rotation des clés de signature JWT (RS256)

1. Générer une nouvelle paire de clés RSA 2048+ bits PKCS#8.
2. Définir le nouveau `NVBES_IDENTITY_TOKEN_KEY_ID` (ex: `identity-2026-q4`).
3. Mettre à jour `NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM` et
   `NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM`.
4. Mettre à jour la clé publique et le `TOKEN_KEY_ID` sur les Resource Servers
   (`account-service`, etc.).
5. Vérifier que `GET /.well-known/jwks.json` expose la nouvelle clé et que les
   nouveaux access tokens sont acceptés.

### Révocation d'urgence en cas de compromission de compte

1. Ouvrir un dossier dans Platform Operations
   (`catégorie: compromission de compte`).
2. Révoquer immédiatement toutes les sessions actives du principal concerné :

   ```sql
   UPDATE identity_sessions
   SET revoked_at = clock_timestamp()
   WHERE principal_id = $1 AND revoked_at IS NULL;
   ```

3. Révoquer l'ensemble des familles de refresh tokens associées :

   ```sql
   UPDATE identity_refresh_tokens
   SET revoked_at = clock_timestamp()
   WHERE principal_id = $1 AND revoked_at IS NULL;
   ```

4. Déclencher l'envoi d'un email de récupération de mot de passe à
   l'utilisateur.
5. Consigner la commande et le motif dans l'audit Platform Operations.

## Procédures opérateur Email V1

Autorité opérationnelle : le
[runbook de livraison](email-delivery-runbook.md), le
[runbook retard / dead-letter](runbook-email-delayed.md) et le
[GO synthétique borné du 2 septembre 2026](email-production-readiness-2026-09-02.md).
Les producteurs produit, comptes invités et trafic public restent fermés.

### Fenêtre de validation synthétique

1. Confirmer que `EMAIL_SYNTHETIC_SMOKE_ENABLED` vaut `false` hors fenêtre.
2. Ouvrir une fenêtre unique, opérateur-contrôlée, avec un destinataire
   contrôlé et un `run_id` diagnostic.
3. Exiger un message ID non vide, un état final `delivered` et au moins un
   événement fournisseur traité.
4. Fermer la fenêtre sans condition (triggers temporaires détruits, ingress
   privé, smoke désactivé), même si une étape antérieure a échoué.

### Suppressions et rejeu

1. Ouvrir un dossier Platform Operations (`catégorie: délivrabilité`).
2. Consulter le snapshot opérations Email (file, échecs, suppressions).
3. Exécuter le rejeu ou la levée de suppression via le service opérations
   authentifié, avec motif et acteur audités.
4. Ne jamais contourner le worker ni appeler TEM depuis un produit.

### Retard de livraison

1. Suivre le [runbook retard](runbook-email-delayed.md) : profondeur de file,
   âge du plus ancien message, erreurs provider, DLQ.
2. Après épuisement des retries, laisser le message en `dead_letter` sans
   bloquer le reste de la file.
3. Pause des producteurs non critiques si les taux de hard-bounce ou de
   plainte dépassent les alertes Grafana Email.

## Ajout futur d'opérateurs

Le modèle prépare des permissions distinctes `support_agent`, `risk_reviewer`,
`billing_operator` et `security_admin`. Aucun nouveau rôle n'est attribué sans
moindre privilège, MFA, séparation lecture/action et audit. Le compte
`platform_owner` ne doit jamais être partagé.
