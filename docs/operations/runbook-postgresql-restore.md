# Runbook — Restauration PostgreSQL et Exercice de Résilience

## 1. Objectifs de Reprise et Cadre Légal

- **RPO (Recovery Point Objective)** : perte maximale autorisée de **24 heures**.
- **RTO (Recovery Time Objective)** : rétablissement complet en **moins de 8 heures**.
- **Isolation stricte** : un exercice de validation ou une restauration de test ne doit JAMAIS impacter les bases actives de production ni exposer de données personnelles (respect strict RGPD).
- **Règle de sortie** : si la durée de restauration dépasse 8h ou si un test d'intégrité échoue, la release ou l'exercice est un **NO-GO immédiat**.

## 2. Procédure de Restauration Étape par Étape

1. **Identification du backup éligible** :
   - Lister les snapshots gérés Scaleway Database pour le domaine concerné (Identity, Account, Billing, Email ou Trust/Risk) :
     ```bash
     scw rdb backup list
     ```
   - Vérifier que l'horodatage du backup le plus récent date de moins de 24 heures (validation RPO).
2. **Création d'une instance cible isolée** :
   - Restaurer le snapshot sélectionné vers une nouvelle ressource PostgreSQL temporaire éphémère.
   - Enregistrer l'heure exacte de lancement (`T_start`).
3. **Attente du provisionnement et mesure du temps** :
   - Suivre l'état de l'instance jusqu'au passage à `ready`.
   - Calculer `Duration = T_ready - T_start`. Vérifier que `Duration <= 8 heures` (validation RTO).
4. **Vérification d'intégrité et de schéma** :
   - Exécuter les vérifications de schéma SQLx sans exécuter de requêtes destructives :
     ```bash
     sqlx migrate info --database-url "$RESTORE_TARGET_DATABASE_URL"
     ```
   - Valider la présence des tables clés, des contraintes d'unicité et des index.
   - Pour Identity : vérifier la présence du principal synthétique de référence.
5. **Nettoyage et journalisation** :
   - Détruire l'instance cible isolée immédiatement après validation.
   - Enregistrer le résultat dans le cockpit Platform Operations via la fiche de validation de restauration.

## 3. Critères de Réussite

- RPO démontré <= 24h.
- RTO mesuré <= 8h (généralement < 15 minutes sur Scaleway Serverless SQL).
- Aucune donnée corrompue ou manquante sur les entités critiques.
- Reçu d'audit complet consigné avec identifiant de l'opérateur et heure.
