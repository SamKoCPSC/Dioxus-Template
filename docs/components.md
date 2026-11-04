# Component showcase and browser CI

## Preview the foundations

Start a normal local development instance with `just up` or `just host-up NAME`,
then follow **Components** in the footer or visit `/dev/components`. Sign-in is
not required. The showcase exercises the actual components in
`src/components/base/`, rather than copies of their markup:

- Primary, secondary, ghost, danger and disabled buttons.
- Controlled inputs and cards that reflect their value.
- Theme color tokens, light/dark appearance, loading and empty states.
- Success/error notifications and their dismissal.
- Confirmation dialogs and start/end drawers, including keyboard navigation.

Examples change only in-memory page state and the browser’s existing theme
preference. They never write account data. Use these examples when changing a
new app’s tokens, typography or component styles. `assets/app.css` contains the
shared styles; the spinner respects reduced-motion preferences.

The showcase requires both **debug assertions enabled** and the existing,
validated `APP_ENV=local` loopback configuration. Its example implementation is
excluded from release builds. The server rejects its path (including a trailing
slash/encoded equivalent) with 404 in a hosted or release app. The client wrapper
and footer apply the same policy using the public local-origin hydration context;
hiding the footer alone is not the access boundary. There is no deployment flag
to enable this page. The route declaration remains as an unavailable stub in
release clients because route matching is shared between server and browser.

## Browser coverage in CI

The `browser` job in `.github/workflows/ci.yml` runs on pull requests and main
pushes. It installs pinned Dioxus CLI 0.7.9, the locked Rust dependencies and the
Playwright version in `requirements-dev.txt`. Playwright installs its matching
Chromium. PostgreSQL, mock OAuth and the GitHub shim use the existing digest-pinned
Docker images. No hosted OAuth clients, production database, bucket, GitHub write
permission or repository secrets are needed.

`scripts/ci-browser.py` creates a uniquely named disposable host-mode instance,
waits for readiness, runs `scripts/browser-smoke.py --showcase`, and tears down
that instance in a `finally` block, including after a failed check. Host mode
checks process/resource ownership before cleanup. A preexisting instance is not
claimed. The runner’s startup logs and the app’s private runtime files are never
printed or uploaded. A failure during startup reports the failure without dumping
those files; reproduce locally to investigate build/runtime problems.

The suite covers synthetic sign-in, owner isolation, saved-note CRUD, Home and
Dashboard freshness, literal/body Search and browser history, profile/provider
settings, native logout/revoked-session replay, navigation drawers, and the
showcase’s interactions on desktop/mobile. It checks loading visibility, reduced
motion, both drawer placements, dialogs, notifications and responsive layout.
This is local mock-OAuth coverage, not proof of a hosted deployment or live
Google/GitHub login. Unit tests separately cover the hosted-page 404 policy.

CI retains only the suite’s PNG captures and `result.json` for seven days, including
on failure. Screenshots contain synthetic data. Do not broaden the artifact paths
to include app logs, runtime directories, browser storage, traces or environment
files. To review a run, download **browser-results** from its Actions summary and
inspect the captures and reported failed phase.

Run the same entry point locally with Docker, the documented host tools and
Chromium installed:

```bash
python3 scripts/ci-browser.py --output ../work/browser-ci
# Managed cloud: read docker/codex-cloud/README.md and inspect readiness first.
python3 scripts/ci-browser.py --managed --dx /PATH/TO/dx \
  --chromium /PATH/TO/chromium --output /workspace/browser-ci-artifacts
```

Or run `just ci-browser`. Outputs must be outside the checkout. To exercise an
already running host instance without starting another one:

```bash
python3 scripts/browser-smoke.py --host --instance NAME --showcase \
  --output ../work/browser-captures
```

Every new app should retain this coverage and adapt the Notes-specific checks to
its own example workflow. When adding a primitive, add an interactive example and
browser checks that verify its useful behavior. Keep the server’s development
gate intact, and do not treat a component being visible as authorization for any
server action.
