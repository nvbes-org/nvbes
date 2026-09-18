# Billing Production Infrastructure

Environnement de production dédié et isolé pour le service Billing V1.

## Environnement GitHub `production-billing`

Le job `deploy-billing` de `.github/workflows/deploy.yml` et le job
`validate-billing-restore` de `.github/workflows/validate-restore.yml`
dépendent de l'environnement protégé `production-billing`.

Variables (`vars`):

- `SCW_ORGANIZATION_ID`, `SCW_PROJECT_ID` — projet Scaleway production Billing ;
- `TERRAFORM_STATE_BUCKET` — bucket de state commun créé par le bootstrap ;
- `GRAFANA_OTLP_ENDPOINT` — collecteur OTLP Grafana Cloud ;
- `BILLING_APP_URL` — URL publique injectée dans `TF_VAR_app_url`
  (URLs de redirection Billing).

Secrets (`secrets`):

- `BILLING_TERRAFORM_STATE_ACCESS_KEY` et `BILLING_TERRAFORM_STATE_SECRET_KEY`
  — identité state dédiée produite par `infrastructure/bootstrap/production`
  (backend S3-compatible Scaleway, clé `production/billing/terraform.tfstate`)
  ; leur absence fait échouer `terraform init` avec
  « No valid credential sources found » ;
- `SCW_ACCESS_KEY` et `SCW_SECRET_KEY` — identité de déploiement
  minimal-privilege pour les ressources Scaleway ;
- `STRIPE_SECRET_KEY` et `STRIPE_WEBHOOK_SECRET` — clés Stripe test mode ;
- `BILLING_METRICS_TOKEN` — token de scraping des métriques Prometheus ;
- `BILLING_OPERATOR_TOKEN` — token des APIs opérateur ; il sert aussi de
  `TF_VAR_billing_grpc_token`, le token gRPC utilisé par billing-worker ;
- `BILLING_SENTRY_DSN` — DSN Sentry du projet Billing ;
- `IDENTITY_TOKEN_PUBLIC_KEY_PEM` — clé publique RSA d'Identity, injectée
  dans `TF_VAR_identity_public_key_pem` pour vérifier les JWT d'accès.

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
