# Set up a new application

## Prerequisites

Local development needs Git, Python 3.11+, just, and a working Docker daemon with
Compose v2. Check `git --version`, `python3 --version`, `just --version`,
`docker info`, and `docker compose version`. On macOS/Windows start Docker Desktop
or an equivalent Docker-compatible runtime. On Linux use Docker Engine/Compose.
The supported application target is web; containers run Linux tooling.

If just is unavailable, invoke `bash scripts/dev-environment.sh up my-app-dev`
directly. For host-side checks also install Rust stable, rustfmt, Clippy, the
wasm32-unknown-unknown target, Dioxus CLI 0.7.9 and PostgreSQL 17 server/client
binaries. CI supplies these tools. On Debian/Ubuntu PostgreSQL tools may need
`$(pg_config --bindir)` added to PATH.

Managed cloud executors additionally require the cloud development guide.
Do not substitute ordinary `just up` for the documented proxy/CA adaptations.

## Generate and name the app

Use GitHub's template-generation flow to create a new repository in your account,
then clone that new URL. Confirm `git remote -v` points to it. Alternatively copy
tracked template files into a new repository with fresh history; never copy .git,
.env files, caches, browser state, local volumes or validation artifacts.

```bash
python3 scripts/configure.py --name "My App" --slug my-app
```

The command validates public metadata, updates app.json and Dioxus.toml, and does
not access the network or read/write credentials. Display identity is embedded
at build time. The Rust package/binary remains `app`; this avoids unnecessary
renaming across Rust, lockfiles, image output paths, and tests. Rebuild to see a
name change.

## Start the isolated development environment

```bash
just up instance=my-app-dev
```

Use a unique name per application when sharing a Docker daemon. Omitting the name
allocates a random one. Each invocation owns its database, storage, network,
caches, source-worktree lock, session-cookie name and dynamic loopback ports.
Only one active instance may bind-mount a given worktree; concurrent development
uses separate Git worktrees. No .env.local is required or rewritten. The generated
private runtime environment lives under the printed runtime path with mode 0600.
Do not print that file.

The command builds the tool image once, applies the declarative schema, and
starts the hot-reloading app, PostgreSQL, MinIO, and synthetic OAuth providers.
The first toolchain build can take many minutes; inspect progress with the
instance's logs. It stays attached and prints endpoints after readiness.
Always use the printed canonical application URL for OAuth.

```bash
just dev-instances
just dev-status instance=my-app-dev
just dev-logs instance=my-app-dev
just down instance=my-app-dev
```

Ctrl+C or the exact down command deletes that disposable instance's data and
caches. Never target another instance or use blanket Docker/volume pruning.

## Alternative: run Dioxus on the host

With the host prerequisites above installed, `just host-up my-app-dev` starts only
PostgreSQL and mock OAuth containers, applies the baseline schema, then runs
Dioxus CLI 0.7.9 on the host. It prints a canonical loopback URL after compilation.
It runs detached; use `just host-status my-app-dev`, `just host-browser my-app-dev`
and `just host-down my-app-dev`. The down command removes that instance's local
data. Host mode does not start MinIO; storage remains disabled.

Use one development mode per worktree: stop a full-container instance before host
mode, and vice versa. They have separate lifecycle locks. Host state/logs are
private and live outside the checkout; do not dump them because OAuth requests
and configuration can contain credentials. `APP_HOST_STATE_ROOT` can select an
external state directory. Use an exclusive `CARGO_TARGET_DIR` to reuse your own
host compilation cache. Managed executors must follow the host-mode section of
the cloud guide for approval and trust setup.

## First successful workflow

1. Open the application, choose Sign in, then Continue with Google.
2. The local mock provider lets you use a synthetic identity. Complete its form.
3. Create a uniquely labeled test note; save, reopen, edit and delete it.
4. Change your profile display name and verify it persists after reload.
5. In Accounts link the mock GitHub provider, then verify at least one method
   remains linked. The local GitHub fixture deliberately has one fixed identity;
   use mock Google subjects for multiple independent synthetic accounts.
6. Sign out and confirm `/notes/new` redirects to login.
7. Run `just browser instance=my-app-dev` for a repeatable browser smoke check.
   Browser artifacts go outside the repository by default. Inspect desktop/mobile
   screenshots; a screenshot existing on disk is not proof it was reviewed.

Run `just check`, `just lint`, `just test` and `just config-test` before publishing
changes. Database tests spawn disposable PostgreSQL processes and never use a
live app database. Host command examples assume the prerequisite tools are on PATH.

## Move to hosting

Follow docs/deployment.md to register real OAuth clients, provision the new app's
resources and configure its HTTPS origin. Copy values from .env.production.example
into the hosting secret manager; placeholder values are not usable credentials.
Local mock credentials must never be used for a hosted application.

## Troubleshooting

- Docker socket denied in a managed environment: use the execution tool's normal
  approval flow; do not start a competing daemon or change storage drivers.
- Network/TLS failure: inspect active policy and CA/proxy readiness. Do not disable
  TLS verification or route around denied hosts.
- Worktree locked or duplicate instance: inspect ownership/status and use another
  name/worktree, or explicitly tear down only your old disposable instance.
- Sign-in returns to a different loopback alias: use the printed canonical URL;
  cookies are host-only and localhost/127.0.0.1 do not share them.
- Slow first startup: inspect logs and available disk; compiler/cache setup is
  expected on a cold machine. Reuse compatible immutable tool images.
- Hosted startup rejected: check the named configuration variable, role separation,
  HTTPS origins and provider endpoints. Error messages intentionally omit values.
