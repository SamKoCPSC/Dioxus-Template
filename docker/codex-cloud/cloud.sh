#!/usr/bin/env bash
# Cloud-only entry point; never source generated instance.env into this shell.
set -euo pipefail
cloud_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
root="$(cd "$cloud_dir/../.." && pwd -P)"
export APP_CLOUD_CA="${APP_CLOUD_CA:-/etc/ssl/certs/ca-certificates.crt}"
# Always use this managed executor's daemon, preserving DOCKER_CONFIG/auth.
unset DOCKER_CONTEXT DOCKER_TLS DOCKER_TLS_VERIFY DOCKER_CERT_PATH
export DOCKER_HOST=unix:///var/run/docker.sock

usage() {
    echo 'Usage: docker/codex-cloud/cloud.sh doctor|prepare|tools|build|up|down|status|logs|browser [instance]'
    echo 'Default instance: cloud. Set APP_CLOUD_STATE_DIR to reuse external generated state.'
}
[[ $# -gt 0 ]] || { usage; exit 1; }
operation="$1"; shift
instance="${1:-cloud}"
instance_id="$(bash "$root/scripts/dev-environment.sh" normalize-instance "$instance")"
export APP_CLOUD_STATE_DIR="$(python3 "$cloud_dir/prepare.py")"

prepare() {
    [[ -r "$APP_CLOUD_CA" ]] || { echo "Cloud CA bundle is unreadable: $APP_CLOUD_CA" >&2; exit 1; }
    python3 "$cloud_dir/prepare.py" --refresh-proxy >/dev/null
}
compact() {
    local image="$1" old
    old="$(docker image inspect --format '{{.Id}}' "$image")"
    python3 "$cloud_dir/compact-image.py" "$image" "$image-compact"
    docker tag "$image-compact" "$image"
    if [[ "$old" != "$(docker image inspect --format '{{.Id}}' "$image")" ]]; then
        # No force: never remove an image still used by a container.
        docker image rm "$old"
    fi
}
finish_build() {
    if [[ "$(docker info --format '{{.Driver}}')" == vfs ]]; then
        # This path requires an exclusive build window (see README).
        docker builder prune --all --force
        compact "$1"
    fi
    df -h "$root"
}
case "$operation" in
    doctor)
        python3 - <<'PY'
import json
from pathlib import Path
policy = json.loads(Path('/etc/codex/network-policy.json').read_text())
if policy.get('version') != 1:
    raise SystemExit('Unsupported network policy snapshot; inspect environment_status')
http = policy.get('http_network_policy', {})
hosts = {rule.get('host') for rule in http.get('egress_rules', [])}
if http.get('type') != 'unrestricted':
    for host in ('storage.googleapis.com',):
        if host not in hosts:
            raise SystemExit(host + ' is missing from the active snapshot; use reviewed platform configuration, not a bypass')
print('Policy snapshot includes required hosts; also check environment_status enforcement/version.')
PY
        docker info --format 'Docker driver: {{.Driver}}'
        curl --fail --silent --show-error --max-time 30 --cacert "$APP_CLOUD_CA" --output /dev/null \
            --write-out 'Cargo registry HTTP %{http_code}\n' https://index.crates.io/config.json
        df -h "$root"
        ;;
    prepare) prepare; echo "$APP_CLOUD_STATE_DIR" ;;
    tools) prepare; python3 "$cloud_dir/runtime-tools.py" "$APP_CLOUD_STATE_DIR" ;;
    build)
        prepare
        exec 9>"${TMPDIR:-/tmp}/app-codex-cloud-build-${UID}.lock"
        flock 9
        mkdir -p "$APP_CLOUD_STATE_DIR/logs"
        # Sequential order is intentional on a 32 GB vfs filesystem.
        docker build --secret "id=cloud_ca,src=$APP_CLOUD_CA" --file "$APP_CLOUD_STATE_DIR/Postgres.Dockerfile" \
            --tag "app-codex-postgres:$instance_id" "$root/docker/postgres" \
            >"$APP_CLOUD_STATE_DIR/logs/postgres-build.log" 2>&1
        finish_build "app-codex-postgres:$instance_id"
        docker build --secret "id=cloud_ca,src=$APP_CLOUD_CA" --file "$APP_CLOUD_STATE_DIR/Development.Dockerfile" \
            --tag "app-development:$instance_id" "$root" \
            >"$APP_CLOUD_STATE_DIR/logs/development-build.log" 2>&1
        finish_build "app-development:$instance_id"
        docker build --build-arg "DEVELOPMENT_IMAGE=app-development:$instance_id" \
            --file "$APP_CLOUD_STATE_DIR/Tools.Dockerfile" --tag "app-codex-tools:$instance_id" "$cloud_dir" \
            >"$APP_CLOUD_STATE_DIR/logs/tools-build.log" 2>&1
        finish_build "app-codex-tools:$instance_id"
        python3 "$cloud_dir/runtime-tools.py" "$APP_CLOUD_STATE_DIR"
        ;;
    up)
        prepare
        for image in "app-development:$instance_id" "app-codex-postgres:$instance_id" "app-codex-tools:$instance_id"; do
            docker image inspect "$image" >/dev/null || { echo "Missing $image; run cloud.sh build $instance first" >&2; exit 1; }
        done
        for tool in esbuild wasm-bindgen tailwindcss; do
            [[ -x "$APP_CLOUD_STATE_DIR/runtime-tools/$tool" ]] || { echo 'Run cloud.sh tools first' >&2; exit 1; }
        done
        APP_STARTUP_TIMEOUT="${APP_STARTUP_TIMEOUT:-2400}" bash "$APP_CLOUD_STATE_DIR/dev-environment.sh" up "$instance"
        ;;
    down|status|logs) bash "$APP_CLOUD_STATE_DIR/dev-environment.sh" "$operation" "$instance" ;;
    browser) python3 "$cloud_dir/browser-smoke.py" --instance "$instance_id" ;;
    *) usage; exit 1 ;;
esac
