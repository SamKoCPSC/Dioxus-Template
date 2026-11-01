# Guide the user through a new app

Read the root AGENTS.md before executing this runbook. The goal is a separately
owned, named application with demonstrated local behavior and a clear deployment
handoff. Do not stop at listing environment variables.

## 1. Establish the destination and public identity

Inspect the remote, current branch, worktree changes and available tooling. If
this is still the template repository, help the user generate a new repository
before changing product code. Preserve existing changes. Confirm the new app's
name and lowercase slug, intended hostname, hosting choice, and license. Bundle these
non-secret questions; do not ask the user to paste any secrets into chat.

Default to this web stack, private Notes example, Google/GitHub login, plain
PostgreSQL, and storage disabled unless the user needs a different scope. Explain
that a new product's roles, billing, password login or native app distribution
would be additional implementation work.

Run the configure command with the user's public name/slug and inspect its diff.
The Rust binary is intentionally named `app`; Docker artifact paths stay stable.
Do not copy history or runtime files from Noms or another application. Do not
reuse an existing application's external resources.

## 2. Prove local setup before requesting provider configuration

Run the environment-specific prerequisite checks in docs/new-app-setup.md.
In a managed executor use docker/codex-cloud/README.md, including readiness and
network policy checks, proxy/CA trust, disk inspection and image reuse. Execute
within the user's authorization using the normal tool approval mechanisms.

Start a named disposable instance such as `my-app-dev`. Use its printed canonical
origin consistently. Complete local synthetic Google sign-in and the Notes
create/save/reopen/edit/delete workflow. Verify profile updates, account linking,
logout and protected-page behavior. Run the browser smoke script and inspect its
desktop/mobile screenshots. Run applicable code/configuration checks. Report
failures accurately and fix reproducible template defects on a branch.

Local mock providers use synthetic identities and must remain local. They are a
way to finish application setup while the user completes external account steps.
They do not validate real Google/GitHub registration or deployment.

## 3. Walk through external account-console steps

Use docs/deployment.md and .env.production.example. Give the user exact callback
URLs using their chosen HTTPS hostname:

- Google: `https://HOST/auth/google/callback`
- GitHub: `https://HOST/auth/github/callback`

The current baseline configures both providers. Have the user create distinct
OAuth clients for each app/environment and enter credentials directly in their
hosting secret manager or supported environment secret bindings. Explain each
value and where it comes from; never request or display credential values.

For PostgreSQL, provision a new database and distinct schema-owner/runtime roles.
For object storage, leave it disabled until needed; if enabled, provision a new
bucket/keys and decide whether the content is public. For reverse proxies, select
the documented client-IP trust policy deliberately. Keep APP_ENV=production and
host-only cookies on hosted services. Registration/configuration may be guided
without authorization to modify the user's external accounts directly.

## 4. Prepare, deploy, then validate the actual environment

Make deployment files and instructions concrete for the chosen provider. Use the
production entrypoint's separate-role checks, migrations and runtime grants.
Build/publish the new repository's image using the manual workflow when authorized;
image publishing is distinct from deploying it. Request a final deployment action
only if it lies outside the user's existing authorization, with the complete
reviewable result ready first.

After a deployment, test the HTTPS home/health endpoints, real provider login,
profile, example CRUD and logout with synthetic data. Validate cookie metadata
without values. Delete test data and private browser storage state afterward.
The Noms-specific staging login is deliberately absent; do not add an unrestricted
test-login endpoint to solve registration or network problems.

## 5. Leave an actionable handoff

Copy docs/setup-checklist.md into the user's chosen project handoff location and
record public metadata, commands and test outcomes only. Include the repository,
commit, chosen hosting, public origin, active modules, what was verified, and the
next user action for any pending secret-manager/console work. Never place a secret
value, session token, cookie, full environment dump or authentication-state file
in the handoff. Explicitly mark live OAuth/deployment unverified when blocked.

Preserve the reusable auth and infrastructure tests as product features replace
the Notes example. Future agents should be able to continue without reconstructing
setup decisions from chat history.
