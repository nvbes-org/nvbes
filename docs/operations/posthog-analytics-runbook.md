# PostHog Analytics Runbook

## Scope

PostHog est utilise pour product analytics, web analytics consenties, feature
flags non critiques, experiments, surveys/feedback, session replay, heatmaps,
error tracking navigateur, logs et traces OTLP via Alloy.

PostHog ne doit jamais etre utilise pour autoriser l'acces, appliquer une regle
de securite, appliquer le billing enforcement, stocker des logs d'audit ou
traiter des donnees de fichier.

## Residency

- Cible par defaut: PostHog EU Cloud `https://eu.i.posthog.com`.
- Alternative production: proxy first-party vers PostHog EU.
- Interdit: endpoint US direct pour les donnees produit ou OTLP.

## Consentement v3

Les finalites PostHog sont separees:

- `posthog_product_analytics`
- `posthog_autocapture_heatmaps`
- `posthog_session_replay`
- `posthog_surveys_feedback`
- `posthog_error_tracking`
- `posthog_feature_flags`

Migration v2 vers v3: un ancien consentement PostHog active uniquement
`posthog_product_analytics`. Replay, surveys, error tracking, heatmaps et flags
demandent un choix explicite v3.

Retrait: le runtime browser stoppe capture/replay/surveys/flags polling et purge
cookies/storage PostHog.

## Variables

Browser:

```bash
VITE_POSTHOG_KEY=
VITE_POSTHOG_HOST=https://eu.i.posthog.com
VITE_ANALYTICS_ID_SALT=
```

Serveur product analytics:

```bash
NVBES_POSTHOG_ENABLED=false
NVBES_POSTHOG_HOST=https://eu.i.posthog.com
NVBES_POSTHOG_PROJECT_TOKEN=
NVBES_ANALYTICS_ID_SALT=
```

Alloy OTLP PostHog:

```bash
POSTHOG_PROJECT_TOKEN_FILE=/etc/nvbes/secrets/posthog_project_token
POSTHOG_OTLP_HOST=https://eu.i.posthog.com
POSTHOG_OTLP_LOGS_ENDPOINT=https://eu.i.posthog.com/i/v1/logs
POSTHOG_OTLP_TRACES_ENDPOINT=https://eu.i.posthog.com/i/v1/traces
```

## Taxonomie Serveur

Evenements allowlistes:

- `auth.signup_completed`
- `auth.email_verified`
- `workspace.created`
- `file.upload_started`
- `file.upload_completed`
- `share_link.created`
- `member.invited`
- `billing.checkout_started`
- `billing.subscription_activated`
- `billing.payment_failed`

Properties allowlistees:

- `country`
- `plan_code`
- `status`
- `workspace_type`
- `file_count`
- `upload_count`
- `size_bytes_bucket`
- `share_link_count`
- `member_count`

Les UUID user/workspace sont HMAC avec `NVBES_ANALYTICS_ID_SALT`; ne jamais
envoyer email, nom, nom de fichier, object key, URL, token, payload ou UUID brut.

## Dashboards A Creer Dans PostHog

- Acquisition: signup started/completed, email verified.
- Activation: workspace created, first upload completed, first share link,
  first invite.
- Billing: checkout started, subscription activated, payment failed.
- Reliability UX: web vitals, browser exceptions, replay consented links.
- Feature flags: exposure par flag/variant, sans flag critique.

## Verification

- Sans consentement browser: aucune requete PostHog.
- Avec finalite unique: seul le module correspondant emet.
- Routes sensibles: replay, heatmaps, surveys et error tracking bloquent auth,
  MFA, privacy export/delete, billing checkout, tokens et fichiers/previews.
- Serveur: `NVBES_POSTHOG_ENABLED=false` doit etre un no-op.
- Alloy: traces passent par redaction + tail sampling; logs passent par
  redaction OTLP; metrics ne partent pas vers PostHog.
