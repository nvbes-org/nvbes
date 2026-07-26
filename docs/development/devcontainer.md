# Dev Container Development Environment

This repository supports a full-stack Dev Container environment for local or remote development.

## Host Requirements

Use one of:

- Docker Desktop or a Docker-compatible runtime with Dev Containers support.
- Coder, DevPod, Codespaces, or another runner that supports the Dev Container specification.

No local Node.js, pnpm, Rust, PostgreSQL, Redis, OpenTofu, or SQLx CLI installation is required inside this workflow.

## Included Tooling

The `workspace` container includes:

- Node.js 25.8.0
- pnpm 11.1.3
- Rust stable with `rustfmt` and `clippy`
- `sqlx-cli` with PostgreSQL support
- OpenTofu
- Go, Python, PostgreSQL client tools, Redis tools, ShellCheck, `ripgrep`, `jq`, `curl`, and build tooling
- Native Rust FFI dependencies for SAML/XML security: `libxml2`, `xmlsec1`, OpenSSL, `pkg-config`, and `libclang`

## Included Services

The Dev Container Docker Compose stack starts:

- PostgreSQL 17 on host port `localhost:15432`
- Redis Stack on host port `localhost:16379`
- RedisInsight on host port `localhost:18001`
- Prometheus on host port `localhost:19090`
- Mimir on host port `localhost:19009`
- Tempo on host port `localhost:13200`
- Alloy on host port `localhost:12345`
- Alloy OTLP gRPC on host port `localhost:14317`
- Alloy OTLP HTTP on host port `localhost:14318`
- Alloy Faro receiver on host port `localhost:12347`
- Alloy Pyroscope ingest on host port `localhost:14040`
- Loki on host port `localhost:13100`
- Pyroscope UI/API on host port `localhost:14041`
- Grafana on host port `localhost:13000`

Sentry and PostHog are not started by the Dev Container. Configure external
Sentry/PostHog endpoints in `.env` only when application capture is needed.
The local Grafana stack is available without external credentials. Use Alloy
for OTLP, Faro and profile ingestion:

```bash
NVBES_OTLP_ENDPOINT=http://127.0.0.1:14317
NVBES_PROFILING_ENABLED=true
NVBES_PROFILING_ENDPOINT=http://127.0.0.1:14040
VITE_FARO_URL=http://localhost:12347/collect
VITE_FARO_TRACING_ORIGINS=http://localhost:4000,http://localhost:3001
```

PostgreSQL initializes the local application databases:

- `nvbes`
- `nvbes_drive`

## First Start

Open the repository in the Dev Container. The `post-create` step installs workspace dependencies and registers Lefthook.

Then run:

```bash
pnpm db:migrate
pnpm dev
```

Application URLs:

- Cloud web: `http://localhost:5173`
- Account web: `http://localhost:3001`
- Account service: `http://localhost:4000`
- Cloud service: `http://localhost:4002`
- Grafana: `http://localhost:13000`

## Sentry and PostHog

The Dev Container does not provide local Sentry or PostHog services. Keep these
values empty in `.env` to disable application capture, or point them at external
Sentry/PostHog deployments:

```bash
VITE_SENTRY_DSN=
VITE_SENTRY_TRACES_SAMPLE_RATE=0
SENTRY_DSN=
SENTRY_TRACES_SAMPLE_RATE=0
VITE_POSTHOG_KEY=
VITE_POSTHOG_HOST=
NVBES_POSTHOG_ENABLED=false
NVBES_POSTHOG_HOST=
NVBES_POSTHOG_PROJECT_TOKEN=
NVBES_ANALYTICS_ID_SALT=
```

Server-side Sentry is enabled when `SENTRY_DSN` is set. Server-side PostHog is
enabled when `NVBES_POSTHOG_ENABLED=true`, `NVBES_POSTHOG_PROJECT_TOKEN` is set,
and `NVBES_ANALYTICS_ID_SALT` is set.

## Environment File

If `.env` does not exist, `.devcontainer/post-create.sh` creates it from `.env.example` and adjusts service hostnames for container networking:

- PostgreSQL host: `postgres`
- Redis host: `redis`

The script does not overwrite an existing `.env`.

When running inside the Dev Container, Compose sets `NVBES_DEVCONTAINER=true`. The shared environment loader then overrides database and Redis endpoints for container networking, so an existing host-oriented `.env` with `localhost` does not break `pnpm dev`.

The post-create step also installs a shell startup guard for stale VS Code JavaScript debug bootloader preloads. If `NODE_OPTIONS` points to a deleted `ms-vscode.js-debug/bootloader.js`, the guard unsets it so Node-based commands such as `pnpm` can start normally.

## Persisted Editor, Cache, and Signing State

The workspace service uses named Docker volumes for editor, tooling, cache, and signing state that should survive container rebuilds:

- VS Code Server user data, installed extensions, extension global storage, and workspace storage: `/home/vscode/.vscode-server`
- VS Code Insiders Server equivalent state: `/home/vscode/.vscode-server-insiders`
- User config used by Git, GitHub CLI, VS Code-compatible tools, and code-server-compatible config paths: `/home/vscode/.config`
- Codex auth, settings, local caches, and plugin state: `/home/vscode/.codex`
- SSH keys and config used by Git remotes and SSH commit signing: `/home/vscode/.ssh`
- GPG keys, trust database, and GPG agent state used by GPG commit signing: `/home/vscode/.gnupg`
- User data used by code-server-compatible tools and some extensions or CLIs: `/home/vscode/.local/share`
- Runtime caches used by language servers, extension helpers, browsers, and CLIs: `/home/vscode/.cache`
- Rust, pnpm, `node_modules`, and `target` caches are also persisted by dedicated project volumes.

Workspace settings stored in the repository, such as `.vscode/settings.json`, are already persisted by the repository bind mount at `/workspaces/nvbes`.

Persisting signing keys keeps them available across rebuilds. GitHub still marks commits as verified only when the public key used for signing is registered on the GitHub account and Git signs commits with that key.

For SSH signing, the post-create step automatically configures Git to use `~/.ssh/id_ed25519.pub` or `~/.ssh/id_rsa.pub` when present. The matching public key must be added to GitHub as a signing key, not only as an authentication key.

Useful checks inside the Dev Container:

```bash
git config --global --get commit.gpgsign
git config --global --get gpg.format
git config --global --get user.signingkey
ssh-add -l
gpg --list-secret-keys --keyid-format=long
```

## GitHub, SSH, and GPG Setup

Run these commands inside the Dev Container. The generated state is persisted by the Docker volumes mounted on `/home/vscode/.ssh`, `/home/vscode/.gnupg`, and `/home/vscode/.config`.

### Configure Git Identity

```bash
git config --global user.name "Your Name"
git config --global user.email "your-email@example.com"
git config --global init.defaultBranch main
```

Use the same email address that is verified on GitHub, or use the GitHub noreply address shown in the GitHub email settings.

### Create and Register an SSH Key

Create a persistent SSH key:

```bash
ssh-keygen -t ed25519 -C "your-email@example.com" -f ~/.ssh/id_ed25519
chmod 700 ~/.ssh
chmod 600 ~/.ssh/id_ed25519
chmod 644 ~/.ssh/id_ed25519.pub
```

Start the agent and load the key for the current shell:

```bash
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519
```

Authenticate GitHub CLI, then register the key for both Git authentication and SSH commit signing:

```bash
gh auth login
gh auth setup-git
gh ssh-key add ~/.ssh/id_ed25519.pub --title "nvbes devcontainer auth" --type authentication
gh ssh-key add ~/.ssh/id_ed25519.pub --title "nvbes devcontainer signing" --type signing
```

Configure Git to sign commits with the SSH key:

```bash
git config --global gpg.format ssh
git config --global user.signingkey ~/.ssh/id_ed25519.pub
git config --global commit.gpgsign true
```

Check the setup:

```bash
ssh -T git@github.com
git config --global --get gpg.format
git config --global --get user.signingkey
```

### Create and Register a GPG Key

GPG is the OpenPGP implementation used here for PGP-style commit signing. Create a persistent GPG key:

```bash
gpg --full-generate-key
```

Recommended prompts:

- Key type: RSA and RSA or ECC.
- Key size: `4096` for RSA.
- Expiration: choose a real rotation window, such as `1y` or `2y`.
- Name and email: match the Git identity configured above.

Find the secret key ID:

```bash
gpg --list-secret-keys --keyid-format=long
```

The key ID is the value after `sec rsa4096/` or `sec ed25519/`. Export and register the public key on GitHub:

```bash
gpg --armor --export YOUR_KEY_ID > /tmp/nvbes-gpg-public-key.asc
gh gpg-key add /tmp/nvbes-gpg-public-key.asc --title "nvbes devcontainer gpg"
rm /tmp/nvbes-gpg-public-key.asc
```

Configure Git to sign commits with GPG instead of SSH:

```bash
git config --global gpg.format openpgp
git config --global user.signingkey YOUR_KEY_ID
git config --global commit.gpgsign true
```

VS Code forwards the host GPG agent into the devcontainer. Let the host
Pinentry application request the passphrase. Do not add `pinentry-mode loopback`
to `~/.gnupg/gpg.conf`: the forwarded agent uses a restricted socket that
rejects loopback mode.

```bash
gpg-connect-agent 'GETINFO version' /bye
echo "test" | gpg --clearsign
```

If a previous setup added `pinentry-mode loopback`, remove that line from
`~/.gnupg/gpg.conf` before retrying the signature. A version warning can appear
when the host and container GPG versions differ; it does not block signing when
the host agent accepts the signing request.

Check the Git setup:

```bash
git config --global --get gpg.format
git config --global --get user.signingkey
```

Use either SSH signing or GPG signing as the active Git signing mode. The last `git config --global gpg.format ...` command determines which mode Git uses.

### Verify a Signed Commit

Create a signed test commit on a throwaway branch:

```bash
git switch -c test/signed-commit
git commit --allow-empty -S -m "test: verify devcontainer signing"
git log --show-signature -1
```

Push the branch and check the commit badge on GitHub:

```bash
git push -u origin test/signed-commit
```

If GitHub still shows `Unverified`, check that:

- The public signing key was added to the same GitHub account that authored the commit.
- The commit email is verified on GitHub.
- `git log --show-signature -1` reports a valid local signature.
- SSH signing keys were added with `--type signing`, not only `--type authentication`.

Delete the test branch after verification:

```bash
git switch main
git branch -D test/signed-commit
git push origin --delete test/signed-commit
```

### Recommended VS Code Extensions

The Dev Container already installs the project baseline extensions from `.devcontainer/devcontainer.json`:

- `biomejs.biome`
- `bradlc.vscode-tailwindcss`
- `dbaeumer.vscode-eslint`
- `esbenp.prettier-vscode`
- `ms-azuretools.vscode-docker`
- `rust-lang.rust-analyzer`
- `tamasfe.even-better-toml`
- `vadimcn.vscode-lldb`

Useful optional extensions for this workflow:

- `github.vscode-github-actions` for workflow files.
- `github.vscode-pull-request-github` for pull requests and GitHub reviews.
- `eamodio.gitlens` for commit and blame inspection.
- `redhat.vscode-yaml` for Compose, GitHub Actions, and infrastructure YAML.
- `timonwong.shellcheck` for shell script diagnostics.
- `ms-vscode-remote.remote-containers` on the host machine to open the repository in the Dev Container.

## Resetting Local Data

Inside the Dev Container:

```bash
pnpm dev:drive-db:reset
pnpm db:migrate
```

To remove all container volumes from the host, stop the Dev Container stack and remove the `nvbes_*` Docker volumes.

## Coder

Coder should consume the same `.devcontainer/devcontainer.json` rather than maintaining a separate development image. This keeps local, remote, and agent workspaces aligned.
