# Security contracts

Preserve these properties when extending the template. They describe intended
behavior and focused test coverage, not a claim that every future product is
covered by a comprehensive security audit.

## Authentication and authorization

- Google/GitHub identity is matched by provider and stable subject. A matching
  email never automatically merges accounts. Linking requires the admitting
  user's existing verified session and a separate OAuth transaction.
- OAuth state is short-lived, single-use and bound to a browser-held secret by
  digest. Validate provider/purpose/redirects, keep transaction cookies scoped,
  and retain PKCE/browser binding checks and rate limits.
- Sessions are server-side rows referenced by signed JWTs, with a 15-minute
  inactivity lifetime and rolling refresh. Revocation/expiry is checked on each
  authenticated request. Cookie defaults are host-only, HttpOnly, SameSite=Lax,
  path `/`, and Secure outside explicit local development.
- Logout is a native POST with the exact canonical Origin; it revokes the session
  row and expires the cookie. Link/unlink operations preserve at least one sign-in
  method; concurrent unlink operations serialize on the user row.
- Private UI routes and server functions are separate enforcement points. Mutations
  require the captured form account to match the current verified session, an
  exact canonical Origin and POST, then revalidate/lock the session through commit.
  Query every resource with its owner ID. A client-supplied UUID is never authority.
- Long-lived forms stay bound to the initiating account. Sign-in refresh does not
  transfer their entries to another user; a session change hides/inerts the form
  and provides a recovery notice. Async completion must not overwrite a different
  account's client state.

## Deployment

APP_ENV=local is exclusively for loopback development; Railway deployment metadata
rejects that mode. Hosted startup requires valid HTTPS origins/endpoints, strong
independent secrets and separate database roles. The entrypoint applies schema
and runtime grants, then removes migration credentials before exec. The runtime
role cannot administer or own the database/schema. Use a dedicated database and
secrets per app and environment.

Client IP policy must reflect the actual trusted ingress. `proxy-chain` trusts
forwarded headers only from configured direct peer IPs/CIDRs. `railway-edge`
requires Railway metadata and its overwritten X-Real-IP header; it assumes every
private-network caller reaching the app is trusted. Do not use that policy with
untrusted private callers or a public raw TCP proxy. Rate limits are in memory
per replica; a multi-replica app needing global limits requires a shared limiter.

No production test-login endpoint is included. Noms' staging bootstrap was
bound to one approved deployment and cannot be made generic by copying its flag
or loosening its guards. Use local synthetic OAuth for development and actual
registered providers for hosted smoke tests. A new staging-only bootstrap needs
an independently reviewed deployment boundary and secret binding.

## Data, media, and artifacts

Do not print or commit secrets, session cookies/tokens, browser storage state,
full environment dumps, production data or deployment runtime files. Configuration
errors name variables without echoing values. Browser/test artifacts belong outside
tracked source and must be reviewed before sharing.

The optional object-store adapter generates owner-scoped UUID keys and validates
small WebP uploads. Its URLs are public media; they do not inherit the Notes API's
privacy. Private documents require private buckets and authenticated/signed delivery.
Adding uploads also requires database attachment references, ownership checks,
quotas, cleanup/retry handling and content validation appropriate to that feature.
Do not let a client submit arbitrary deletion keys or treat a URL as proof of ownership.

## Changes and validation

Keep the auth/session/proxy/isolation regressions, both compiler targets, Clippy,
dependency audit and tracked-secret checks in CI. Update the tests when a generic
contract changes. New product roles, multi-tenancy, external content import,
billing or storage workflows need their own threat/authorization model.

The dependency audit uses narrowly justified, expiring exceptions with active
graph checks. Read [dependency-audit.md](dependency-audit.md) before changing
features, cache key types, or the audit policy.
