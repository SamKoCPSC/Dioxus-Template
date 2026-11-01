# Managed cloud development

Read the installed cloud-environment-runtime skill first, including its Docker
and networking references. Call environment_status and inspect the selected
executor's /etc/codex/network-policy.json. Require current observations and an
enforced network policy; inspect the actual request error if access fails.
Do not dump environment variables, credentials, Docker registry auth, runtime
instance.env files or browser storage state.

## Inspect and prepare

Check Linux amd64, Python 3.11+, Bash, Git, curl, Docker/Compose, flock and available
disk. Host Cargo/Rust is needed when the pinned wasm-bindgen helper must be built.
Browser checks need Python Playwright and an installed Chromium. Docker socket,
browser or proxy access can require the execution tool's normal approval flow.
A sandbox denial is different from a network-policy denial. Never bypass policy,
start a competing daemon, change its storage driver or disable TLS verification.

Inspect git status, docker images/containers and `df -h .` before building. A
32 GB `vfs` daemon needs an exclusive cold-build window; do not overlap builds.
The `build` operation prunes the default builder's entire unused build cache on
vfs, then compacts only its built images. Use it only when those caches are not
shared with unrelated work; never blanket-prune images or data volumes.

Run from the template/generated app root:

```bash
export APP_CLOUD_STATE_DIR=/workspace/app-cloud-state/my-app
bash docker/codex-cloud/cloud.sh doctor
bash docker/codex-cloud/cloud.sh prepare
bash docker/codex-cloud/cloud.sh build my-app-dev
bash docker/codex-cloud/cloud.sh up my-app-dev
```

The state directory must be outside the source checkout. The helper derives paths
from the checkout and supports any generated app name. It generates adapted
Dockerfiles, Compose config and lifecycle scripts externally, refreshing Docker's
proxy address while preserving credentials and other contexts/settings. Networked
build steps mount the current public CA bundle as a BuildKit secret; runtime tools
receive a read-only CA mount. Helper downloads use pinned versions/checksums.
The default CA bundle is /etc/ssl/certs/ca-certificates.crt; override APP_CLOUD_CA
only when the platform supplies another combined trust bundle.

## Reuse retained compatible images/tools

Prefer existing verified images over a cold build. Inspect their provenance,
architecture, executable/tool versions and Dockerfile compatibility first. Required
runtime tools are Dioxus CLI 0.7.9, the current lockfile's Rust-compatible toolchain,
pgschema 1.12.1 and the Dockerfile-pinned MinIO/mc builds. Schema-init uses numeric
UID 1000. The PostgreSQL image is the pinned plain PostgreSQL 17 base.

After compatibility is established, tag the actual matching image references:

```bash
docker tag VERIFIED_DEVELOPMENT_IMAGE app-development:app-my-app-dev
docker tag VERIFIED_POSTGRES_IMAGE app-codex-postgres:app-my-app-dev
docker tag VERIFIED_TOOLS_IMAGE app-codex-tools:app-my-app-dev
bash docker/codex-cloud/cloud.sh tools
bash docker/codex-cloud/cloud.sh up my-app-dev
```

SOURCE placeholders above must be replaced with verified local references; these
commands do not pull or publish arbitrary prebuilt application images. `up` requires
all images/helpers and never initiates automatic builds. Bootstrap roots are
removed before final creation to reduce vfs disk use. It retains the ordinary
lifecycle's source lock, ownership labels/tokens, dynamic loopback ports and
separate writable instance volumes.

When an executor has a **template-owned external host build cache**, the optional
APP_CLOUD_CARGO_TARGET environment variable can bind that existing directory at
/app-target. It must be outside the source checkout and exclusive to this app.
Do not share another active instance's mutable target directory. Leave it unset
for normal per-instance named volumes. To reuse host compilation profiles set
APP_CARGO_PROFILE_DEV_CODEGEN_UNITS consistently with the host's profile (256 for
Rust's default). The override avoids adding container-only linker flags to that
host cache. Review compiler/target/profile compatibility; a cache is not evidence
of a fresh successful check.

## Release-bundle optimizer

Release bundling also invokes wasm-opt. Dioxus CLI 0.7.9's downloader does not
honor the managed session's extra CA roots. Prefetch the pinned optimizer using
the inherited proxy and trusted CA, verify its checksum before execution, and
place its `bin/wasm-opt` and `lib/` files beneath
`$DX_HOME/tools/binaryen-129/`. The cache name is 129 in this CLI even though its
Linux download URL selects Binaryen 127; recheck both when upgrading the CLI.

Validated Linux amd64 archive:
`https://github.com/WebAssembly/binaryen/releases/download/version_127/binaryen-version_127-x86_64-linux.tar.gz`

SHA-256:
`c8ebe5d00c978601086bcad33b2c80fcfe33d6a8b87b754ba4ea86a9a16cc316`

Extract only the needed regular files into the owned external tool cache. Keep
the executable/library layout intact and TLS validation enabled. Do not copy
session trust files or private runtime configuration into the release image.

On a constrained vfs executor, an isolated compatible builder with an executable
memory-backed Cargo target can avoid compiler output filling Docker's disk.
Check available memory first, keep an exclusive build window and copy the final
bundle out before stopping that builder. This does not replace validation of the
full multi-stage Dockerfile on an adequately sized builder. See
[local production results](../../docs/extraction-validation.md) and the local
production validation section of [deployment.md](../../docs/deployment.md).

## Host-run fallback for constrained executors

When compatible host Rust and Dioxus tools are already available but `vfs`
container creation exhausts disk, use the smaller host-run path. Inspect disk,
containers and images first; retain unrelated data. Reuse the pinned PostgreSQL,
mock-OAuth and nginx images. No development/tool container or MinIO is created.

```bash
python3 scripts/host-dev.py up my-app-dev --managed --dx /PATH/TO/dx
python3 scripts/host-dev.py status my-app-dev
python3 scripts/browser-smoke.py --host --instance my-app-dev \
  --chromium /PATH/TO/chromium --output /workspace/app-browser-artifacts/my-app
python3 scripts/host-dev.py down my-app-dev
```

Use the execution tool's Docker/browser approval mechanism when socket or proxy
access requires it. `--managed` selects the managed daemon socket while preserving
proxy and Docker registry configuration. Host Cargo/dx/helper downloads must use
the session proxy and trusted CA bundle from the runtime skill. Do not disable TLS
or route around a blocked destination. An already prepared `DX_HOME` may reuse
compatible pinned Dioxus helpers; an exclusive `CARGO_TARGET_DIR` reuses only this
app's own cache. Install host PostgreSQL binaries for native tests.

Host mode is detached after readiness and uses private state beneath `/tmp` by
default; its logs can contain OAuth query strings and must not be printed. It has
an ownership-checked exact teardown command. It uses a separate worktree lock from
the full-container mode; never run both in one worktree concurrently. The default
local database credentials and provider identities are disposable synthetic data,
not suitable for hosted use. This path verifies the actual web app but does not
validate production image building or deployment.

## Browser validation and teardown

`up` stays attached. Keep its execution session alive. From another terminal:

```bash
python3 -m pip install -r requirements-dev.txt
APP_BROWSER_OUTPUT=/workspace/app-browser-artifacts/my-app \
  bash docker/codex-cloud/cloud.sh browser my-app-dev
bash docker/codex-cloud/cloud.sh status my-app-dev
bash docker/codex-cloud/cloud.sh down my-app-dev
```

The browser smoke script permits only a ready loopback instance, uses synthetic
mock OAuth, tests private Notes CRUD and logout, and writes captures/results outside
the repository. Use an installed Chromium; do not download a browser through a
blocked destination or ignore TLS errors. Inspect relevant desktop/mobile screenshots
and display them with the client's image mechanism when reporting visual evidence.
A file sitting on the executor is not automatically visible to the user.

The stack is a local development environment with synthetic OAuth and local
credentials. Hosted deployment uses docs/deployment.md and separate resources.
No public preview, port forward, live provider credential or Noms staging binding
is needed for the local smoke test.
