# Initial extraction validation

Validated locally on 2026-10-31 from the source commit recorded in provenance.md.
These are extraction results, not guarantees for a subsequently modified app.

## Completed

- Native server regression suite: 244 tests passed, including disposable PostgreSQL
  identity/session tests, owner-scoped Notes, mutation account binding, session
  revocation, configuration/proxy policies and maintenance cleanup.
- WASM compilation and strict Clippy on server/WASM passed. The optional
  `server,storage-s3` compilation check passed.
- Twelve Python setup/credential/host-lifecycle/audit-policy tests, four managed-cloud adaptation
  tests, 39 isolated-development configuration checks and six schema-plan fixture
  checks passed. Shell syntax, formatting and tracked-secret checks passed.
- Strict dependency audit passed with the two expiring reachability exceptions
  documented in dependency-audit.md. Compatible updates replaced the affected
  Tokio and yanked enumset/spin versions.
- Changed public name/slug with configure.py, rebuilt and passed the OAuth cookie
  metadata regression, then restored the template defaults.
- Real local Chromium against the running app: synthetic Google OAuth, return-path
  preservation, 15-minute local cookie metadata, private note create/save/reopen/
  edit/delete, profile persistence, explicit mock GitHub link/unlink, last-method
  guard, theme interaction, native logout and rejected revoked-cookie replay.
- Desktop and 390px mobile editor/dialog screenshots were captured and inspected.
  No horizontal overflow or browser page errors occurred. Captures and browser
  results remain outside the repository; no authentication state was serialized.
- The isolated template-owned test stack and synthetic database were removed.
  No source-app data volume was reused or deleted.

## Limits and follow-up

The full development-container path could not start in the constrained managed
executor because Docker's `vfs` copy-on-create exhausted its disk allowance.
The documented host-run path was used successfully with isolated Docker
PostgreSQL/mock providers. A fresh release bundle and production runtime image
were subsequently built and tested locally on 2026-11-01, as recorded below. The full cold multi-stage build
and image publishing remain unverified. No Railway deployment or live
Google/GitHub registration was changed or tested.
Optional object storage was compiled, not tested against a live bucket; it remains
a building block requiring product-specific attachment ownership and cleanup.

CI workflows are provided for pull requests. Inspect their actual results before
merging. Enable GitHub's template setting once main contains the reviewed
template. No merge or hosted deployment was performed.

A source license has not been selected. Resolve licensing before distributing
code under a chosen license; do not silently add a license in a generated app.

## Local production-mode deployment — 2026-11-01

Built the locked application with Dioxus CLI 0.7.9 and Rust 1.97.1 in a retained
Bookworm-compatible builder. Compiler output used a 6 GiB executable tmpfs because
Docker vfs duplicated several GiB just to create the build container. The fresh
release bundle and verified pgschema 1.12.1 binary were supplied to the Dockerfile's
production runtime stage; the declared Debian Bookworm runtime base, application
entrypoint, migrations, non-root user and healthcheck were exercised. The generated
runtime-stage Dockerfile added only managed-proxy CA handling to networked apt
commands and replaced the builder-stage COPY inputs with those verified outputs.
This validates release packaging/runtime behavior, not the cold Rust/Go tool
installation and cargo-chef stages of the complete multi-stage build.

A real startup failure exposed private checkout permissions: migration files were
0600 and their source directory 0700, so the non-root app could not read them.
The Dockerfile now grants read/traversal permissions to shipped files while
removing group/other write access. The same private source files then passed
pre-deploy preparation, startup and an idempotent preparation on restart.

The disposable deployment used a separate PostgreSQL 17 volume with distinct
schema-owner and restricted runtime roles, synthetic Google/GitHub OAuth, and a
local TLS proxy. A short-lived local CA was trusted by both the browser and app;
certificate verification was enabled throughout. Production mode remained enabled.
Docker's proxy defaults needed explicit NO_PROXY/no_proxy selections for only the
owned local provider hostname. No live Noms, Railway, provider client or media
resource was used.

Passed runtime assertions: non-root user, healthy container, restricted role
attributes and identity, denied runtime CREATE TABLE, owner-owned schema, real
runtime database connections, migration credentials absent from the app process,
readable migrations and non-writable application/binary.

Twelve Chromium workflow checks passed: public/theme hydration, guest gating,
synthetic OAuth, host-only Secure/HttpOnly/SameSite=Lax session cookie with path /
and Max-Age 900, HSTS, private Notes create/save/reopen/edit/delete, private cache
headers, authenticated data/session persistence across a restart, profile save,
GitHub link/unlink, mobile dialog/layout, native logout and rejected revoked-cookie
replay. No browser page errors occurred. Desktop/mobile captures were inspected;
artifacts remain outside Git. The owned fixture, database and private credentials
were cleaned up after validation.

After the runtime fix, the required repository checks were rerun successfully:
`just check`, `just lint`, `just test` (244 native and 12 Python tests),
`just config-test` (39 lifecycle checks and four cloud tests), six schema-plan
fixture checks, the tracked-secret scan and `just audit` with the existing two
expiring reachability exceptions.
