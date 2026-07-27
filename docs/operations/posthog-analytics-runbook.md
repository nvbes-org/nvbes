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

## Environnement De Developpement

Le fichier racine `.env` est charge par les scripts `pnpm dev:*` et reste
ignore par Git. Pour activer un projet PostHog EU dedie au developpement:

```bash
NVBES_POSTHOG_ENABLED=true
NVBES_POSTHOG_HOST=https://eu.i.posthog.com
NVBES_POSTHOG_PROJECT_TOKEN=<project-token>
NVBES_ANALYTICS_ID_SALT=<secret-aleatoire-32-caracteres-minimum>
VITE_POSTHOG_KEY=<project-token>
VITE_POSTHOG_HOST=https://eu.i.posthog.com
VITE_ANALYTICS_ID_SALT=<meme-secret>
POSTHOG_SOURCEMAP_UPLOAD_ENABLED=false
POSTHOG_CLI_PROJECT_ID=<project-id>
POSTHOG_CLI_HOST=https://eu.posthog.com
```

Le token navigateur est un project token public. Comme toute variable `VITE_*`,
le sel navigateur est visible dans le bundle: il sert uniquement de namespace de
pseudonymisation, jamais de secret d'autorisation. Sa valeur reste alignee avec
le serveur pour produire les memes identifiants pseudonymes. Ne jamais committer
`.env`.

En developpement, un consentement PostHog actif sans `VITE_POSTHOG_KEY` produit
une erreur explicite dans la console. Cote serveur, l'activation sans token ou
sans sel fait echouer la validation de configuration au demarrage.

L'upload des source maps reste desactive en developpement. Pour un build de
release, `POSTHOG_SOURCEMAP_UPLOAD_ENABLED=true` exige explicitement
`POSTHOG_CLI_API_KEY`, `POSTHOG_CLI_PROJECT_ID` et un identifiant de release
(`NVBES_RELEASE`, `VITE_NVBES_BUILD_ID` ou `GITHUB_SHA`).

Services:

- `account-service` construit le sink serveur PostHog au bootstrap quand
  `NVBES_POSTHOG_ENABLED=true`.
- `cloud-service`, `billing-service` et `billing-worker` utilisent le meme
  adaptateur batch PostHog.
- `account-web` utilise `VITE_POSTHOG_KEY`, `VITE_POSTHOG_HOST` et
  `VITE_ANALYTICS_ID_SALT`; l'initialisation PostHog est lazy et ne demarre
  qu'apres consentement analytics/PostHog.
- `cloud-web` utilise le meme runtime partage et le meme modele de consentement.
- La CSP Identity ajoute `VITE_POSTHOG_HOST` dans `connect-src`; sans host
  explicite, le fallback browser reste `https://eu.i.posthog.com`.

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
- `auth.login_completed`
- `auth.logout_completed`
- `auth.session_revoked`
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

Apres consentement product analytics, le client HTTP propage
`X-PostHog-Distinct-Id` et `X-PostHog-Session-Id`. Les serveurs valident ces
valeurs, rattachent leurs evenements a la session PostHog et conservent les
groupes workspace pseudonymes. Ces headers sont limites a l'origine API
configuree et restent absents sans consentement, sur une route sensible ou vers
une URL tierce.

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
- Le payload `/batch/` doit placer `distinct_id` dans `properties`.
- Les exceptions navigateur consenties doivent apparaitre dans PostHog Error
  Tracking sans activer l'autocapture ni les pageviews automatiques.
- Alloy: traces passent par redaction + tail sampling; logs passent par
  redaction OTLP; metrics ne partent pas vers PostHog.
