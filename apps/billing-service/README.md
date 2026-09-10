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
  `NVBES_IDENTITY_TOKEN_ISSUER`, `NVBES_IDENTITY_TOKEN_KEY_ID`,
  `NVBES_BILLING_IDENTITY_RESOURCE_CLIENT_ID` et
  `NVBES_BILLING_IDENTITY_RESOURCE_SECRET`. Les deux dernières valeurs doivent
  correspondre à une entrée Billing du registre de ressources Identity.
  L'émetteur est
  comparé exactement, y compris son éventuel slash final. L'ancienne variable
  `NVBES_IDENTITY_PUBLIC_KEY_PEM` n'est plus utilisée par Billing.
- Sans configuration Identity, les API utilisateur refusent tous les bearer tokens ;
  les webhooks conservent leur authentification Stripe indépendante. Une
  configuration Identity partielle ou invalide interdit le démarrage du serveur.
- Aucun UUID ou préfixe `test-` ne permet de contourner l'authentification, même
  en développement. Les tests JWT génèrent une paire RSA éphémère en mémoire.
- Après validation locale du JWT, chaque requête protégée consulte Identity via
  `/oauth/introspect`. Une session inactive reçoit 401 ; un service indisponible,
  une réponse incohérente ou un dépassement de capacité reçoit 503. Aucun cache
  positif, retry ou repli sur la seule signature n'est utilisé.
- Le client borne les consultations à 16 simultanées, 2,5 secondes et 16 Kio de
  réponse. Il refuse les redirections et exige que toutes les claims attendues
  correspondent au token vérifié. Les jetons liés à DPoP restent refusés en Bearer.
- DPoP est accepté avec `NVBES_BILLING_PUBLIC_ORIGIN`, une preuve ES256 liée au
  jeton et la migration 0002 du registre anti-rejeu local. L'introspection Identity
  et l'autorisation Account restent obligatoires. Voir le
  [contrat DPoP des API](../../docs/architecture/identity-resource-dpop.md).
- Après Identity, chaque opération consulte Account pour vérifier le propriétaire
  du compte personnel ou de l'équipe. Configurer `NVBES_BILLING_ACCOUNT_ORIGIN`
  et `NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET`, partagé avec Account et distinct
  des credentials Identity. Sans cette paire, les opérations utilisateur donnent 503. Un refus métier donne 403 ; une panne Account donne 503, sans cache ni repli.
  Voir le [contrat et les routes typées](../../docs/architecture/billing-account-authorization.md).
- Le démarrage local prépare un secret de ressource privé partagé avec Identity
  dans `.temp/dev-runtime`, sans l'afficher. Un registre de ressources personnalisé
  exige des credentials Billing explicites et cohérents. La présence d'un registre
  OAuth côté Identity reste nécessaire pour activer l'endpoint d'introspection.

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
