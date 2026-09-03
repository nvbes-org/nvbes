# nvbes Billing Service (V1)

Service Billing V1 minimal, opérant exclusivement avec Stripe en mode test.

## Capacités
- Association de comptes et équipes Account à des clients Stripe test.
- Catalogue minimal de plans et prix test.
- Création de sessions de Checkout et Customer Portal uniquement en mode test.
- Ingestion et vérification cryptographique des webhooks Stripe (tolérance temporelle 300s, déduplication, rejet livemode).
- Maintien d'un état local minimal des abonnements avec protection contre le désordre des événements.
- File dead-letter et réconciliation manuelle pour l'opérateur solo.
- Journal d'audit append-only et outbox transactionnelle.
- Aucun changement d'accès automatique (aucun enforcement produit direct).

## Sécurité et FinOps
- Interdiction stricte de toute clé API `sk_live_` ou `rk_live_`.
- Rejet immédiat de tout webhook avec `livemode: true`.
- Scale-to-zero : conteneur `min_scale = 0`, `max_scale = 1` ; base Serverless SQL `min_cpu = 0`, `max_cpu = 1`.
- Coût consolidé respectant le plafond global de 30 EUR TTC/mois.

## Actions CLI
```bash
billing-service serve                     # Démarre le serveur HTTP
billing-service migrate                   # Applique les migrations SQLx
billing-service synthetic-billing-smoke   # Exécute le test de fumée synthétique
```
