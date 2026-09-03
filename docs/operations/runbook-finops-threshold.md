# Runbook — Dépassement des Paliers Budgétaires FinOps

## 1. Contrat Budgétaire Canonique

Le coût récurrent total de production nvbes V1 est strictement plafonné à **30 EUR TTC par mois**, toutes charges confondues (serveurs Scaleway, bases de données, registres, email, observabilité, domaines et DNS).
La cible opérationnelle normale est **20 EUR TTC**, avec une marge de sécurité de 10 EUR.

## 2. Matrice des Paliers et Actions Immédiates

| Palier de dépense | Statut / Stage | Action Immédiate |
| :--- | :--- | :--- |
| **15 EUR TTC** | Information | Alerte préventive ; vérification de la tendance de consommation sur 30 jours. |
| **20 EUR TTC** | Cible mensuelle | Revue FinOps complète ; identification des postes en dépassement de cible. |
| **25 EUR TTC** | `disable_non_essential` | **Désactivation automatique** des traitements non essentiels (télémétrie verbeuse, scans secondaires, jobs d'analytics). |
| **28 EUR TTC** | `freeze_cost_creation` | **Gel de création de coûts** : blocage du provisionnement de nouvelles ressources, rétention raccourcie au minimum légal. |
| **30 EUR TTC** | `essential_only` | **Garde-fou strict** : seules les opérations critiques (récupération de compte, emails transactionnels vitaux, ingestion durable des webhooks) restent actives. |

## 3. Procédure d'Intervention Opérateur en Cas de Dépassement

1. **Vérification de la projection mensuelle dans le cockpit** :
   - Consulter le panneau `FinOps Spend & Ceiling Gate`.
   - Identifier le ou les postes responsables (Domain/DNS, Compute, Postgres, Email, Storage, Observabilité).
2. **Contrôle des configurations Scaleway serverless** :
   - Vérifier que chaque conteneur Scaleway a bien `min_scale = 0`.
   - Vérifier qu'aucune instance ne tourne en continu de manière non prévue.
3. **Application des mesures d'urgence** :
   - Si dépense >= 25 EUR : déclencher la procédure dégradée `FinOpsThresholdExceeded` dans le cockpit.
   - Réduire le taux d'échantillonnage Sentry et désactiver les traces superflues.
   - Forcer le nettoyage des images temporaires de registry non épinglées.
4. **Vérification du respect du gate** :
   ```bash
   pnpm check:finops
   ```
   - Tout franchissement non contenu au-delà de 30 EUR TTC entraîne le refus immédiat de tout nouveau déploiement.

## 4. Clôture et Rapport

- Enregistrer les causes du pic de coût dans le journal des incidents.
- Ajuster les quotas applicatifs pour prévenir toute récidive.
- Confirmer que la projection de dépense repasse sous le palier cible de 20 EUR TTC.
