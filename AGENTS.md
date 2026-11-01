# Agent entry point

This is a reusable Rust/Dioxus web-app template, extracted from Noms. It is a
starting repository for independent applications, not a shared production service.

## When the user wants to set up a new app

Read `docs/agent-setup.md` first, then `docs/new-app-setup.md`. Follow its phased
workflow and use `docs/setup-checklist.md` as the handoff record. Ask for product
name/slug and deployment preferences; never ask for credential values in chat.
Make progress on local setup while external account configuration is pending.
The setup guide explains which account-console steps only the user can complete.

- Keep the Rust package/binary named `app`; set product identity with
  `python3 scripts/configure.py --name 'Product Name' --slug product-name`.
  Do not perform a global search-and-replace across Rust code, SQL, tests or locks.
- Inspect the repository remote before pushing: a generated app must target its
  own repository, not `SamKoCPSC/Dioxus-Template`.
- `just up` creates a disposable local stack and prints its canonical URL. Its
  Google/GitHub logins are synthetic mocks; they do not require real credentials.
- Host mode (`just host-up NAME`) is available with host Rust/Dioxus tools; see
  the setup guide. Stop the other mode before switching within one worktree.
- Keep runtime `.env` files, browser storage state, screenshots, tool caches and
  generated cloud adaptations outside tracked source. Never print cookies,
  bearer credentials, the process environment, or secret-file contents.
- In a managed cloud environment, read `docker/codex-cloud/README.md` and the
  installed cloud-environment-runtime skill. Inspect current policy/readiness,
  Docker containers/images and disk before starting/building. Use supported
  execution approvals for Docker/socket/browser access. Do not bypass network
  denials, disable TLS validation, or start a competing Docker daemon.
- Every derived app/environment needs its own database, OAuth clients, secrets,
  media bucket and callback URLs. Never inherit live Noms deployment identifiers,
  account data, cookie domains, or credentials.

## Architecture and changes

Read `docs/architecture.md`, `docs/pages.md` and `docs/security.md`. `src/auth` and middleware are
reusable foundations; `src/pages/notes.rs`, `src/api/notes.rs` and the `notes`
table are the replaceable example. `src/api/workspace.rs` also supplies the
Notes-based Dashboard/Search read models. Keep infrastructure and feature policy apart.
Server functions must authorize on the server; hiding a route is insufficient.
Capture the initiating account for forms, check exact Origin on mutations, and
keep the session lock through the write transaction. Scope every resource query
by its owner and bound inputs, queries and work.

Change `migrations/schema.sql` declaratively and update tests with it. Production
uses distinct migration-owner and restricted runtime roles. The app never owns
its production schema. No third-party PostgreSQL extension is required.

Optional S3 storage is a server adapter, not a complete media-attachment system.
An app adding uploads must implement reference ownership, lifecycle cleanup and
an appropriate public/private delivery policy. See `docs/storage.md`.

## Verification and reporting

Run `cargo fmt --all --check`, both `just check` targets, both `just lint` targets,
`just test`, and relevant configuration tests. Native database tests need
PostgreSQL binaries on PATH. Use focused tests while iterating, then the complete
required checks once the change is ready. For UI/setup changes, run the browser
smoke test against the actual local instance and inspect desktop/mobile captures.
The smoke test deletes only its uniquely labeled example note.

Report which checks actually ran, any remaining user account-console steps, and
any limitations. Distinguish local mock OAuth from live provider validation and
image publishing from an actual deployment. Do not report a successful deployment
or production login from static configuration checks alone. User authorization
for a requested action persists; continue routine reversible work without asking
for repeated confirmations. Use a feature branch/PR for reviewable changes and
never merge or change external account settings without task authorization.
