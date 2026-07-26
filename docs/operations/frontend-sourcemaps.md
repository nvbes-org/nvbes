# Frontend production source maps

The five Vite applications generate hidden source maps for release builds. The maps are uploaded
privately to Sentry, PostHog, and Grafana Cloud Frontend Observability, then removed from each
application's `dist` directory before it becomes a deployable artifact.

## Release identity

`NVBES_RELEASE` is the canonical release identifier shared by all three providers. CI sets it to the
Git commit SHA and exposes the same value to browser runtimes as `VITE_NVBES_BUILD_ID`.

## Required CI configuration

Store API keys and tokens as GitHub Actions secrets. Store project IDs, application IDs, stack IDs,
and endpoints as repository or environment variables.

### Sentry

- `SENTRY_AUTH_TOKEN`
- `SENTRY_ORG`
- `SENTRY_PROJECT_<APP>` for each web application
- Optional `SENTRY_URL` for a self-hosted instance

The application suffix is uppercase with hyphens replaced by underscores, for example
`SENTRY_PROJECT_ACCOUNT_WEB`.

### PostHog

- `POSTHOG_CLI_API_KEY`: personal API key with `error_tracking:write`
- `POSTHOG_CLI_PROJECT_ID`
- `POSTHOG_CLI_HOST`, normally `https://eu.posthog.com`

All applications can share the PostHog project. Releases remain distinct because their release names
are the application names.

### Grafana Cloud Frontend Observability

- `GRAFANA_FARO_SOURCEMAP_ENDPOINT`
- `GRAFANA_FARO_SOURCEMAP_API_KEY`
- `GRAFANA_CLOUD_STACK_ID`
- `GRAFANA_FARO_APP_ID_<APP>` for each web application

The access-policy token must allow `sourcemaps:read`, `sourcemaps:write`, and `sourcemaps:delete`.
The source-map endpoint is distinct from the browser Faro collector URL.

## Failure behavior

A provider is disabled when none of its private build credentials are present, which keeps local
builds usable. A partially configured provider fails the build instead of silently publishing an
unsymbolicated release.

Provider upload failures fail the release build. Grafana upload runs through the Faro CLI after Vite
so that an HTTP failure cannot be swallowed by a bundler plugin. The cleanup command runs only after
Vite and every upload complete successfully:

```bash
node ../../scripts/remove-public-sourcemaps.mjs dist
```

This final cleanup is deliberately provider-independent. It guarantees that `.map` files are absent
from deployable `dist` directories even when no upload provider is configured for a local build.
