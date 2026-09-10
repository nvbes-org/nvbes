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

- Les API utilisateur exigent un jeton d'accès Identity RS256 `at+jwt`, avec
  l'audience exacte `nvbes-billing-service`. Le checkout exige `billing:checkout`
  et la consultation exige `billing:read`.
- Configurer ensemble `NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM`,
  `NVBES_IDENTITY_TOKEN_ISSUER` et `NVBES_IDENTITY_TOKEN_KEY_ID`. L'émetteur est
  comparé exactement, y compris son éventuel slash final. L'ancienne variable
  `NVBES_IDENTITY_PUBLIC_KEY_PEM` n'est plus utilisée par Billing.
- Sans ces trois valeurs, les API utilisateur refusent tous les bearer tokens ;
  les webhooks conservent leur authentification Stripe indépendante. Une
  configuration Identity partielle ou invalide interdit le démarrage du serveur.
- Aucun UUID ou préfixe `test-` ne permet de contourner l'authentification, même
  en développement. Les tests JWT génèrent une paire RSA éphémère en mémoire.
- Les jetons liés à DPoP sont refusés en Bearer. Le contrôle de leur preuve et
  la consultation de l'état de révocation Identity restent à intégrer ; une
  signature valide seule ne prouve pas que la session est encore active.

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
