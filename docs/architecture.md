# Architecture and extension points

## Runtime

`src/main.rs` composes the Axum server, Dioxus SSR/WASM route inventory, request
middleware and shared hydration context. The `app` crate/binary name is stable;
`app.json` supplies public identity through build.rs. Avoid copying an older
app's deployment-specific identifiers when generating another app.

- `src/auth/`: browser-bound OAuth transactions, provider identity resolution,
  session verification/revocation/refresh, account-bound forms, session-scoped
  rendering and safe redirects. Generic behavior was extracted from Noms.
- `src/middleware/`: protected routes, local canonical-origin redirects and
  trusted-proxy/client-IP rate-limit handling. Update the protected-route
  inventory when adding a private page; also authorize its server functions.
- `src/db/`: bounded PostgreSQL pool and identity/session persistence. Queries use
  runtime SQLx bindings, so the starter has no copied product `.sqlx` cache.
- `src/api/`: server functions. Shared mutation checks enforce exact Origin,
  session authentication and the form's captured account. Writes hold a session
  row lock through their transaction so revocation cannot race authorization.
- `src/components/base/`: buttons, cards, inputs, native dialogs, menus, feedback,
  and keyboard/focus lifecycle helpers. Their tests cover interaction logic;
  not every primitive is needed by the example UI.
- `src/components/base/shared_drawer.rs`: route-independent side panel built on
  the shared modal lifecycle; `src/components/navigation.rs` composes app links
  and account actions inside it. See pages.md for usage and dismissal contracts.
- `src/pages/showcase.rs`: interactive component previews compiled into debug
  builds, gated by validated local mode. `middleware/development.rs` returns 404
  in hosted/release apps. See components.md for the showcase and browser CI.
- `src/utils/`: theme persistence and mobile viewport/keyboard helpers.
- `src/storage.rs`: optional server-only S3 adapter, compiled with `storage-s3`.
  See storage.md for its deliberate public-media boundary and extension work.
- `migrations/`: declarative schema and production runtime grants. PostgreSQL 17
  supplies UUID generation; auth/session cleanup runs in the server every five
  minutes instead of requiring pg_cron. Expiry/revocation checks do not depend
  on cleanup succeeding.

## Replace the example feature

The Notes feature is in `src/pages/notes.rs`, `src/api/notes.rs`, `src/types.rs`
(the Note model), and the notes table/index in migrations/schema.sql. The Home,
Dashboard and Search example read models also use Notes through
`src/api/workspace.rs`; replace those queries with the new product’s domain.
The example demonstrates
private ownership checks, bounded fields, manual saving, update/reopen, confirmation
before deletion and recoverable feedback. Lists show the most recent 100 notes.

When adding your real feature, retain those authorization patterns and add suitable
pagination, per-user quotas and concurrency limits for its resource usage. Add
business rules in the feature module; do not embed them in the auth or pool code.
Update the route inventory, protected paths and navigation together. See
[pages.md](pages.md) for the starter routes. Keep the
owner predicate in reads, updates and deletes, and test a different user's access.

Product-specific schemas should evolve additively once deployed. An undeployed
fresh app may replace the sample schema; never delete live tables merely because
an example was removed from the UI. Run schema plan checks and inspect the plan.

## Independence and updates

Generating a repository copies a starting point. Future improvements here do not
silently update generated apps. Review and port foundation fixes deliberately,
with each app's tests. Extract shared published crates only if multiple apps later
need a maintained shared contract. Keep each application's data and credentials
independent even when hosting or infrastructure providers are shared.
