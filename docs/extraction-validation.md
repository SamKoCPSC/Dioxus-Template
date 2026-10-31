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
PostgreSQL/mock providers. The production image was not built or published, and
no Railway deployment or live Google/GitHub registration was changed or tested.
Optional object storage was compiled, not tested against a live bucket; it remains
a building block requiring product-specific attachment ownership and cleanup.

CI workflows are provided but remote execution requires publishing the feature
branch. GitHub rejected repository writes during extraction with HTTP 403
“Resource not accessible by integration.” After access is corrected, publish the
branches, open the review PR, run CI, and enable GitHub's template setting once
main contains the reviewed template. No merge or hosted deployment was performed.

A source license has not been selected. Resolve licensing before distributing
code under a chosen license; do not silently add a license in a generated app.
