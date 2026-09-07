# Validation du rollout CI affected-only

Date de collecte : 7 septembre 2026.

## Mesures GitHub

La [référence complète](https://github.com/nvbes-org/nvbes/actions/runs/34133822650)
réussit toutes les lanes et `ci-gate`, au commit `6e892d59`.

| Périmètre                                     | Durée       |
| --------------------------------------------- | ----------- |
| Premier job démarré jusqu'au dernier terminé  | 16 min 15 s |
| Somme des minutes de runner arrondies par job | 30 min      |
| Job de planification                          | 40 s        |
| Rust                                          | 5 min 31 s  |
| Les cinq suites PostgreSQL                    | 14 min 7 s  |

Cette référence utilise le repli complet après les exécutions précédentes
en échec. Elle précède la correction de l'index SQLite : son taux de cache
Nx est de 0 %. Elle valide les lanes ensemble, sans démontrer les objectifs
de durée pour un changement ciblé.

Le [push limité à la CI](https://github.com/nvbes-org/nvbes/actions/runs/34135398978)
réussit au commit `a4d7e726` : **1 min 34 s** entre le premier et le dernier
job, **5 minutes de runner** arrondies par job, dont **9 s** pour `scope`.
Seuls `authorize-cache`, `scope`, `contracts` et `ci-gate` s'exécutent.
Les cinq lanes de runtime sont ignorées conformément au plan. L'objectif
de deux minutes pour ce parcours est atteint sur cette exécution.

## Défauts corrigés pendant la validation

- Le test PostgreSQL Billing appelait Stripe avec une clé factice. Il utilise
  désormais un serveur HTTP local et vérifie la requête, la réponse et le
  résultat du cycle de vie dans PostgreSQL. Le test ciblé passe sur une base
  jetable ; `cargo check --workspace` passe également.
- Le contrôle de disponibilité PostgreSQL détectait le serveur temporaire
  d'initialisation de l'image Docker. Il attend maintenant le serveur TCP
  définitif. La création des cinq bases isolées a été vérifiée sur un
  conteneur neuf.
- Les jobs annulés avant démarrage n'ont pas de journal GitHub. Le collecteur
  lit les journaux des jobs exécutés et conserve les coûts des annulations.
  Un journal manquant pour un job exécuté reste une erreur.
- Copier les résultats Nx sans leur index SQLite provoquait des cache misses.
  Le namespace v3 conserve l'index avec les résultats, daemon désactivé.
  La synchronisation considère les dates de modification, car la taille
  d'un index SQLite peut rester constante après modification.

## Preuves locales

Les planificateurs réels `scope-preflight.mjs` et `nx-cache-manager.mjs` ont
été exécutés dans des dossiers temporaires sans `node_modules` :

| Diff historique      | Résultat                               | Temps local |
| -------------------- | -------------------------------------- | ----------- |
| `f591f245..16482484` | Docs seules, aucune lane de validation | 140 ms      |
| `202d23f1..f591f245` | CI seule, lane contrats uniquement     | 135 ms      |

Ces durées locales ne sont pas des mesures de runner GitHub.

Un test d'intégration utilise le moteur natif de la version Nx installée :
le transfert des seuls résultats entre deux dossiers indépendants ne produit
pas de cache hit ; le transfert des résultats avec l'index restitue le code
de sortie et la sortie console attendus.

## Conditions d'activation restantes

Le mode reste `shadow`. L'activation Rust ciblée exige vingt PR distinctes,
au moins sept jours d'observation, aucune divergence et une revue opérateur.
Un push du workspace complet ne remplace pas une comparaison de périmètre
strictement réduit sur PR.

Les protections de branche GitHub restent bloquées par un HTTP 403 lié à
l'offre du dépôt privé. Aucun changement d'offre ni de visibilité n'est prévu.
Les gains à cache chaud et par famille de changements doivent être mesurés
avant de déclarer les objectifs de performance atteints.
