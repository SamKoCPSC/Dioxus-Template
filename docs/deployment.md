# Deploy a generated app

Configure the new repository's app name/slug first, pass local verification, and
choose an HTTPS hostname. The Dockerfile and Railway config are examples that
can be used by the new application; creating a repository does not provision or
attach any hosted resources automatically.

## 1. Provision independent resources

Create a new PostgreSQL 17 database. Use a schema-owning migration/deployment role
and a separate application runtime role on that same server/database. For example,
in the provider's trusted SQL console the database owner can create:

```sql
CREATE ROLE app_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOINHERIT NOREPLICATION NOBYPASSRLS;
```

Set its password directly in that console/secret workflow (psql's `\password
app_runtime` avoids embedding it in SQL history). The runtime role must not own
the database, public schema or application objects, and must have no role
memberships. The migration role needs permission to apply schema and grants.
Enter their separate URLs as DATABASE_URL and MIGRATION_DATABASE_URL in hosting.
Use database TLS as appropriate to the provider; never disable certificate checks
to work around connectivity.

Generate a new random SESSION_SECRET with at least 32 high-entropy bytes and
store it directly in the new environment's secret manager. Do not paste it into
chat or commit it. Use independent values in each app/environment. Leave
COOKIE_DOMAIN unset for host-only cookies.

## 2. Register OAuth providers

This baseline configures Google and GitHub. Create separate registrations for the
new app and for each staging/production environment. Do not reuse Noms' clients.

Google Cloud Console → APIs & Services → OAuth consent screen and Credentials:
create a Web application OAuth client, set the authorized HTTPS origin, and add:

```text
https://YOUR_HOST/auth/google/callback
```

For a testing consent screen, add the intended test accounts and ensure its
publication/verification status matches your audience. Put its client ID and
secret into GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET in hosting.

GitHub → Settings → Developer settings → OAuth Apps → New OAuth App: set the
homepage to the HTTPS app origin and callback to:

```text
https://YOUR_HOST/auth/github/callback
```

Store GITHUB_CLIENT_ID and GITHUB_CLIENT_SECRET directly in hosting. Set the
provider endpoint URLs to the real HTTPS values shown in .env.production.example.
A registration in a console is not proof that the deployed callback works; test
real sign-in after deployment. The local mocks require no external registrations.

## 3. Configure the app service

Use .env.production.example as a variable-name reference. Do not deploy its
placeholder values. Set APP_ENV=production, BASE_URL to the canonical HTTPS origin,
provider settings, database roles/URLs, and SESSION_SECRET. Storage is disabled
by default; see storage.md before enabling it.

For Railway public HTTP ingress, CLIENT_IP_POLICY=railway-edge is supported only
when all private callers able to reach the app are trusted and no raw public TCP
proxy exposes it. Review every workload sharing that private network. For another
reverse proxy use proxy-chain and configure TRUSTED_PROXIES to its actual direct
peer IPs/CIDRs. Never guess a broad network range merely to make a test pass.

The production entrypoint parses database URLs safely, verifies role separation,
applies extensions/schema/grants, and removes migration credentials before starting
the application as a non-root user. No search/vocabulary seed is needed.

## 4. Build and deploy

The GitHub Actions **Publish image** workflow is manual: run it for the generated
app's main branch when image publication is authorized. It runs the application
checks first, then publishes the generated repository's own GHCR image tagged
`main` and `sha-FULL_COMMIT`. Choose `include_storage` to compile the optional
S3 adapter and publish distinct `main-storage` / `sha-FULL_COMMIT-storage` tags;
set STORAGE_ENABLED=true only with that variant. For a direct Docker build,
`--build-arg APP_FEATURES=storage-s3` enables the same module. Package visibility/access must permit the hosting
provider to pull it. No registry credential is committed to this repository.

Alternatively connect the generated repository to Railway using the included
Dockerfile and railway.toml. Set the pre-deploy command to:

```text
/usr/local/app/entrypoint.sh --prepare-database
```

Image-backed Railway services need this command entered in their service settings;
repository-backed services read railway.toml. Startup also prepares the database
as a fallback. Keep the healthcheck at `/healthz`, timeout 300 seconds. Supply
credentials at runtime, never Docker build arguments. APP_FEATURES is a non-secret
build option for optional compiled modules, not a credential channel.

Publishing an image is distinct from deploying it. Record the deployed immutable
image/tag and retain a previous version for rollback. Changes to real hosting,
DNS, OAuth registrations and deployments require authorization within the current
user task; do not infer permission from an earlier unrelated app's setup.

## 5. Validate the actual hosted environment

Check HTTPS home and health, real Google/GitHub sign-in, account linking, profile,
private example CRUD and logout. Verify cookie scope/protections without printing
values and revisit a protected page after logout. Capture/inspect desktop/mobile
screenshots with synthetic data only. Clean up test records and private browser
state. If environment readiness, a secret binding or hostname permission is missing,
report the named blocker without exposing credentials or bypassing the policy.

Desktop/native mobile packaging, billing, teams/roles and live media-upload
workflows require additional product implementation and are not claimed by this
web starter.

## Local production-mode validation

A local deployment test should exercise the release runtime and entrypoint with
APP_ENV=production. Build the image with `docker build --tag my-app:local .` on a
machine with adequate Docker disk and memory. Use a new disposable PostgreSQL
instance with separate owner/runtime roles and a local HTTPS reverse proxy. Keep
synthetic provider endpoints behind trusted local HTTPS too: production validation
rejects HTTP endpoints and development placeholder credentials. Generate synthetic
secrets directly into a private environment file outside the repository.

Trust the local test CA explicitly in the app and an isolated browser profile;
do not turn off certificate validation. Retain managed-cloud proxy/CA settings
for public downloads. If Docker client proxy defaults are configured, explicitly
pass NO_PROXY and no_proxy for the owned local fixture hostnames; env-file entries
can be superseded by those defaults. Preserve other proxy settings and do not add
public destinations merely to route around policy.

Verify the --prepare-database entrypoint, startup/health, non-root identity,
restricted runtime privileges, readable migrations and real browser CRUD/logout.
Check cookie metadata without values. Restart the app and verify preparation is
idempotent and authenticated data persists. Review desktop/mobile screenshots,
then remove the owned fixture's containers, volume, synthetic data, private keys
and browser state. Report any substituted build stages precisely; a successful
runtime smoke test is not proof of a cold image build, provider registration,
image publication or Railway deployment.

See [extraction-validation.md](extraction-validation.md) for the initial local
production test results and the focused private-checkout permission fix.
