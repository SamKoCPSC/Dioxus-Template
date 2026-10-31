# New app setup record

Copy this into the generated app's handoff documentation and fill in public
metadata/outcomes. Never record secrets, cookies, browser storage state or full
process environments. Mark pending work explicitly rather than checking it off.

- Repository / branch / commit:
- Template version / origin commit:
- App display name / slug / license:
- Public origin and hosting choice:
- Enabled modules (Notes example, optional storage, product-specific additions):
- Local instance name / canonical loopback origin:

## Local verification

- [ ] Prerequisites and environment policy/readiness checked
- [ ] Metadata configured and rebuilt
- [ ] Isolated stack ready and schema initialization successful
- [ ] Synthetic OAuth login completed
- [ ] Notes create/save/reopen/edit/delete completed
- [ ] Profile update / account linking / last-method guard checked
- [ ] Logout and protected-route behavior checked
- [ ] Native and WASM checks, formatting, lint and tests passed
- [ ] Browser smoke passed; desktop/mobile screenshots inspected
- [ ] Synthetic data and private browser artifacts cleaned up appropriately

## Hosted setup

- [ ] Separate database and least-privilege runtime role provisioned
- [ ] New Google/GitHub clients registered with exact callback URLs
- [ ] Credentials entered directly into approved secret storage
- [ ] HTTPS BASE_URL and explicit client-IP policy configured
- [ ] Storage disabled or new dedicated bucket/credentials configured
- [ ] Migration pre-deploy/startup hook verified
- [ ] Image build/publishing verified for the new repository
- [ ] Deployment verified independently of image publishing
- [ ] Real provider login, CRUD and logout verified with synthetic data
- [ ] Cookie protections inspected without exposing values

Pending user console actions / blockers:

Next concrete action:

Validation commands, timestamps, and artifact locations (no credentials):
