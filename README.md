# Dioxus Template

A reusable web-application foundation extracted from
[SamKoCPSC/Noms](https://github.com/SamKoCPSC/Noms). It combines Rust, Dioxus SSR and
WebAssembly, Axum, PostgreSQL, OAuth, session handling, and a small private Notes
example. New apps keep their own repository and evolve independently.

## Start a new app

1. Generate a repository from this template (GitHub **Use this template → Create a
   new repository**), or copy it into a new repository with fresh Git history.
2. Clone **your new repository** and run:

   ```bash
   python3 scripts/configure.py --name "My App" --slug my-app
   just up
   ```

3. Open the printed application URL. Use **Continue with Google** for a synthetic
   local sign-in, then create, reopen, edit and delete a note.

Prerequisites: Git, Python 3.11+, [just](https://just.systems/), and Docker with
Compose. Rust/Dioxus/PostgreSQL client tools are installed in the development
image. The first image build can take substantial time; subsequent apps reuse
Docker's toolchain layers. Ports and mutable volumes are isolated per invocation.
The command stays attached; Ctrl+C removes that disposable instance and its data.
Use a unique explicit instance name when running multiple generated apps.

**Working with an agent?** Ask it to read [AGENTS.md](AGENTS.md) and follow the
[agent setup guide](docs/agent-setup.md). The [new app setup guide](docs/new-app-setup.md)
walks through prerequisites, project identity, OAuth, and the first deployment.

Managed Codex cloud environments must use the
[cloud development guide](docker/codex-cloud/README.md) for proxy/CA trust and
reusing retained images. It also documents a host-run option for executors where
Docker copy-on-create disk use prevents a full development container.

## Included

- Google/GitHub OAuth login and explicit account linking; no automatic merging
  by email. Browser-bound OAuth state, PKCE where supported, safe redirects,
  exact-origin logout, and database-backed revocable/rolling sessions.
- Profiles, linked sign-in methods, stale-account form protection and sign-in
  refresh. Native accessible confirmation dialogs, light/dark theme, responsive
  layouts, and reusable interaction primitives.
- Public landing/About pages, signed-in Home and Dashboard, private Search,
  Account overview and a Settings hub with browser-persisted appearance controls.
- Reusable accessible side drawer, used by responsive app navigation, with native
  modal focus/scroll handling and start/end placement.
- Owner-scoped private Notes CRUD as a small feature example.
- Plain PostgreSQL 17, declarative `pgschema` setup, per-test disposable databases,
  and separate production migration/runtime roles.
- Isolated development app/database/MinIO/mock-provider stacks; a server-side
  opt-in S3/R2/MinIO adapter.
- GitHub Actions checks, a production Dockerfile, an explicit image-publishing
  workflow, and a Railway configuration example.

This version targets the web (SSR + WASM). Native desktop/mobile distributions,
password login, teams/roles, billing, and a complete upload lifecycle are not
implemented. Noms' recipe, pantry, search/vocabulary, branding and special staging
login deployment are excluded.

## Commands and documentation

| Task | Command or guide |
| --- | --- |
| Change public identity | `python3 scripts/configure.py --name "My App" --slug my-app` |
| Start local stack | `just up` or `just up instance=my-app-dev` |
| Run app on host + Docker infrastructure | `just host-up my-app-dev` / `just host-down my-app-dev` |
| Host-mode browser workflow | `just host-browser my-app-dev` |
| Inspect/stop an instance | `just dev-status instance=my-app-dev` / `just down instance=my-app-dev` |
| Format / compile / lint | `just fmt` / `just check` / `just lint` |
| Native + Python tests | `just test` |
| Local browser workflow | `just browser instance=my-app-dev` |
| New app setup | [docs/new-app-setup.md](docs/new-app-setup.md) |
| Agent-guided setup | [docs/agent-setup.md](docs/agent-setup.md) |
| Starter pages and search behavior | [docs/pages.md](docs/pages.md) |
| Architecture and extension points | [docs/architecture.md](docs/architecture.md) |
| Production deployment | [docs/deployment.md](docs/deployment.md) |
| Security properties | [docs/security.md](docs/security.md) |
| Optional object storage | [docs/storage.md](docs/storage.md) |
| Extraction origin and scope | [docs/provenance.md](docs/provenance.md) |
| Setup handoff record | [docs/setup-checklist.md](docs/setup-checklist.md) |

The internal Rust crate/binary name stays `app`. Product identity is defined in
`app.json` and is embedded at build time, so naming a new app does not require
rewriting Rust imports, Docker bundle paths or test fixtures.

License: not yet selected. Choose a license before distributing derived source;
see [provenance](docs/provenance.md) for the extraction source.
