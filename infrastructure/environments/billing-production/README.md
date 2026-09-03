# Billing Production Infrastructure

Environnement de production dédié et isolé pour le service Billing V1.

## Architecture FinOps & Résilience

- **Conteneur Serverless**: Scaleway Serverless Container `min_scale = 0`, `max_scale = 1`, protocole HTTP/1, scale-to-zero complet hors trafic.
- **Base de données**: Scaleway Serverless SQL DB `min_cpu = 0`, `max_cpu = 1`, facturation à la seconde avec scale-to-zero.
- **Migrations de schéma**: Job Scaleway dédié avec identité IAM migratrice restreinte.
- **Sécurité des paiements**:
  - Clés `sk_live_` et `rk_live_` strictement interdites et rejetées au démarrage.
  - Webhooks live (`livemode = true`) rejetés avec erreur 400.
  - Aucune carte bancaire stockée localement (déléguée à Stripe Checkout/Portal en test mode).
  - Webhooks idempotents avec table d'historique `billing_webhook_events`.
  - Protection contre les événements désynchronisés (out-of-order) via horodatage Stripe.
  - File de réconciliation manuelle pour opérateur solo.
