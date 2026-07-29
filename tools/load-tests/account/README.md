# Account load tests

These k6 scenarios target an exact allowlisted staging origin or an isolated local
origin. Staging requires both `ACCOUNT_LOAD_ALLOWED_ORIGINS` and the protected
`ACCOUNT_PRODUCTION_DENIED_ORIGINS`. The latter extends built-in production aliases
for Account, API, Backoffice and Cloud; every denied hostname is rejected on every
port, even if an operator accidentally adds it to the staging allowlist. Credentials,
encoded authorities, non-allowlisted ports and internal targets are also rejected.
The repository runner resolves every non-local target immediately before
k6 and rejects the complete answer set if any A or AAAA record is private,
link-local, shared, multicast, documentation-only or otherwise reserved. Resolution
failure and empty or malformed answer sets fail closed.

Public API smoke:

```bash
NVBES_TARGET_ENV=local \
K6_PROFILE=smoke \
K6_SUITE=account-api \
ACCOUNT_SERVICE_BASE_URL=http://localhost:4000 \
bash tools/account-quality/run-account-k6.sh
```

Authenticated read workload:

```bash
K6_PROFILE=load \
ACCOUNT_SERVICE_BASE_URL=https://account.staging.example \
ACCOUNT_LOAD_ALLOWED_ORIGINS=https://account.staging.example \
ACCOUNT_PRODUCTION_DENIED_ORIGINS=https://account.example,https://api.example \
ACCOUNT_TEST_COOKIE='synthetic-session-cookie' \
ACCOUNT_TEST_AUTHUSER=0 \
NVBES_TARGET_ENV=staging \
bash tools/account-quality/run-account-k6.sh
```

Direct staging invocation with `k6 run` is unsupported because k6's portable
JavaScript policy cannot perform the Node DNS preflight. Use the repository runner;
local direct execution remains useful only for script development.

Supported profiles are `smoke`, `load`, `volume`, `spike`, `stress`, `soak` and
`scalability`. Durations are exact profile contracts; a mismatched
`ACCOUNT_K6_DURATION` fails. Use only synthetic accounts. Never print or archive cookie
values.

`setup()` returns no data because k6 serializes its return value under `setup_data`.
The runner verifies every raw summary against the cookie value before it can succeed.

The repository runner uses a digest-pinned k6 container when k6 is not installed:

```bash
NVBES_TARGET_ENV=test \
K6_PROFILE=smoke \
K6_SUITE=account-api \
ACCOUNT_SERVICE_BASE_URL=http://host.docker.internal:4000 \
bash tools/account-quality/run-account-k6.sh
```

`pnpm nx run account-quality:test-resilience` delegates to the same runner and requires
an explicit resilience profile. GitHub Actions runs public synthetic workloads only.

## Scalability evidence is currently blocked

The comparator revalidates the exact metadata schema for all four 1/2/4/8 runs.
It requires `configuredDuration: "15m"`, an immutable hexadecimal release matching
`^[a-f0-9]{7,64}$`, a canonical UTC timestamp, an exact summary pairing, a synthetic
dataset identifier, a positive configured rate and replica count, the staging
environment and one Account suite. Missing and additional metadata properties are
rejected.

That is necessary but not sufficient. `ACCOUNT_K6_REPLICAS` is only an untrusted
workload label. The repository has no selected control-plane provider, authenticated
read adapter, signing identity or trust root, so the comparator deliberately removes
any stale green verdict by overwriting it with an explicit `blocked` artifact, then
exits non-zero after validating the campaign. The
`scalability-comparison.example.json` file is a structural fixture, not evidence that
can produce a passing verdict.

Closing this gap requires a provider-specific verifier with this minimum contract:

- bind every run to a control-plane deployment UID and the same immutable release or
  image digest as `NVBES_RELEASE`;
- obtain desired and ready replica counts from an authenticated control-plane read,
  both immediately before and immediately after the 15-minute run;
- bind provider timestamps, cluster, namespace and deployment identity to the exact
  summary digest;
- verify the provider response online or verify a signed attestation against a
  repository-configured issuer and trust root;
- require both observations to show the requested 1, 2, 4 or 8 replicas ready for
  the corresponding run.

Environment variables, k6 tags and unsigned JSON files cannot satisfy that contract.
Until an infrastructure adapter and trust model are chosen, scalability remains an
explicit gap and no green verdict is emitted.

The authenticated `account-session` suite must run from an isolated trusted environment;
the repository CI policy intentionally forbids injecting a browser session cookie into a
manual workflow.
