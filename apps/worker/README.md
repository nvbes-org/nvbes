# nvbes worker

One binary for every background batch of the V1 monolith. The API stays
strictly serverless; this worker owns migrations, email delivery and RGPD
retention.

## Commands

| Command | Runtime | Behaviour |
|---|---|---|
| `nvbes-worker serve` | Scaleway Container (image default) | Health on `$PORT` (`/health/live`, `/health/ready`, `/metrics`) and one loop per task in `NVBES_WORKER_TASKS` |
| `nvbes-worker run <task>` | Scaleway Job | One batch, a JSON summary on stdout, exit code 1 on failure |

| Task | Kind | Does |
|---|---|---|
| `migrate` | one-shot | Applies identity and account migrations, each in its own schema; idempotent |
| `identity-emails` | recurring (15 s) | Sends queued security notifications and password-recovery links |
| `privacy-retention` | recurring (300 s) | Completes due RGPD exports and account closures |
| `rotate-mfa-key` | one-shot | Re-seals MFA secrets under the active key after a key change |

## Configuration

Outside `development`/`test` every value marked required must be set or the
worker refuses to start. See `.env.example` for the full list.

- `DATABASE_URL` (required), `NVBES_WORKER_DATABASE_MAX_CONNECTIONS` (2)
- `NVBES_WORKER_TASKS` (required for `serve`), `NVBES_WORKER_*_INTERVAL_SECS`
- `NVBES_IDENTITY_MFA_ENCRYPTION_KEY` (required by `identity-emails` and
  `rotate-mfa-key`; must equal the API's key)
- `NVBES_EMAIL_PROVIDER` (`smtp`, `scaleway-tem`; `test-capture` in development),
  `NVBES_EMAIL_FROM_EMAIL`, SMTP or Scaleway TEM credentials

## Local development

```bash
pnpm dev:worker
```

Runs `run migrate`, then `serve`, with the Identity keys shared with the API.
