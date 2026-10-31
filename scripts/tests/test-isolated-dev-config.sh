#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
DEV_SCRIPT="$REPO_ROOT/scripts/dev-environment.sh"
COMPOSE_FILE="$REPO_ROOT/docker-compose.yml"
TEST_TMP="$(mktemp -d)"
LOCK_OWNER_PID=""

cleanup() {
    if [[ -n "$LOCK_OWNER_PID" ]]; then
        kill -TERM "$LOCK_OWNER_PID" 2>/dev/null || true
        wait "$LOCK_OWNER_PID" 2>/dev/null || true
    fi
    rm -rf "$TEST_TMP"
}
trap cleanup EXIT

PASS=0
FAIL=0

pass() {
    echo "  PASS: $1"
    PASS=$((PASS + 1))
}

fail() {
    echo "  FAIL: $1"
    FAIL=$((FAIL + 1))
}

assert_eq() {
    local description="$1"
    local expected="$2"
    local actual="$3"
    if [[ "$actual" == "$expected" ]]; then
        pass "$description"
    else
        fail "$description (expected '$expected', got '$actual')"
    fi
}

assert_success() {
    local description="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        pass "$description"
    else
        fail "$description"
    fi
}

assert_failure() {
    local description="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        fail "$description"
    else
        pass "$description"
    fi
}

assert_contains() {
    local description="$1"
    local pattern="$2"
    local file="$3"
    if grep -Eq -- "$pattern" "$file"; then
        pass "$description"
    else
        fail "$description"
    fi
}

assert_not_contains() {
    local description="$1"
    local pattern="$2"
    local file="$3"
    if grep -Eq -- "$pattern" "$file"; then
        fail "$description"
    else
        pass "$description"
    fi
}

echo "Template isolated development configuration tests"

if [[ ! -x "$DEV_SCRIPT" ]]; then
    echo "  FAIL: scripts/dev-environment.sh exists and is executable"
    exit 1
fi
pass "scripts/dev-environment.sh exists and is executable"

assert_eq \
    "explicit instance names receive the app- prefix" \
    "app-agent-a" \
    "$("$DEV_SCRIPT" normalize-instance agent-a)"
assert_eq \
    "an existing app- prefix is not duplicated" \
    "app-agent-a" \
    "$("$DEV_SCRIPT" normalize-instance app-agent-a)"
assert_eq \
    "the documented just instance assignment form is accepted" \
    "app-agent-a" \
    "$("$DEV_SCRIPT" normalize-instance instance=agent-a)"
assert_success \
    "lowercase alphanumeric, dash, and underscore names are accepted" \
    "$DEV_SCRIPT" normalize-instance branch_123-a
assert_failure \
    "uppercase instance names are rejected" \
    "$DEV_SCRIPT" normalize-instance Agent-A
assert_failure \
    "shell metacharacters are rejected" \
    "$DEV_SCRIPT" normalize-instance 'agent-a;docker compose down'
assert_failure \
    "empty explicit instance names are rejected by the normalization command" \
    "$DEV_SCRIPT" normalize-instance ""
assert_failure \
    "overlong instance names are rejected" \
    "$DEV_SCRIPT" normalize-instance "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

mkdir -p "$TEST_TMP/fake-bin" "$TEST_TMP/safety-runtime/app-safety-test"
cat >"$TEST_TMP/fake-bin/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

case "${1-}" in
    info)
        exit 0
        ;;
    compose)
        if [[ "${2-}" == "version" ]]; then
            exit 0
        fi
        echo "compose command must not run for a foreign project" >>"$FAKE_DOCKER_LOG"
        exit 0
        ;;
    ps)
        printf 'foreign-container\n'
        ;;
    inspect)
        if [[ "$*" == *com.app.dev-environment* ]]; then
            printf 'false\n'
        else
            printf 'different-instance\n'
        fi
        ;;
    *)
        echo "unexpected docker command: $*" >>"$FAKE_DOCKER_LOG"
        exit 1
        ;;
esac
EOF
chmod +x "$TEST_TMP/fake-bin/docker"
cat >"$TEST_TMP/safety-runtime/app-safety-test/metadata" <<'EOF'
INSTANCE_ID=app-safety-test
PROJECT_NAME=app-safety-test
INSTANCE_TOKEN=expected-token
EOF
: >"$TEST_TMP/fake-docker.log"
if PATH="$TEST_TMP/fake-bin:$PATH" \
    FAKE_DOCKER_LOG="$TEST_TMP/fake-docker.log" \
    APP_RUNTIME_ROOT="$TEST_TMP/safety-runtime" \
    "$DEV_SCRIPT" down safety-test >/dev/null 2>&1; then
    fail "targeted cleanup refuses a Compose project with foreign ownership labels"
else
    pass "targeted cleanup refuses a Compose project with foreign ownership labels"
fi
if [[ -d "$TEST_TMP/safety-runtime/app-safety-test" ]] &&
    [[ ! -s "$TEST_TMP/fake-docker.log" ]]; then
    pass "refused cleanup preserves metadata and never invokes Compose down"
else
    fail "refused cleanup preserves metadata and never invokes Compose down"
fi

mkdir -p "$TEST_TMP/lock-fake-bin"
cat >"$TEST_TMP/lock-fake-bin/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

case "${1-}" in
    info)
        exit 0
        ;;
    ps)
        exit 0
        ;;
    compose)
        if [[ "${2-}" == "version" ]]; then
            exit 0
        fi
        if [[ "$*" == *" create --build"* ]]; then
            touch "$FAKE_LOCK_STARTED"
            while [[ ! -f "$FAKE_LOCK_RELEASE" ]]; do
                sleep 1
            done
            exit 1
        fi
        if [[ "$*" == *" down --volumes --remove-orphans"* ]]; then
            exit 0
        fi
        echo "unexpected compose command: $*" >&2
        exit 1
        ;;
    *)
        echo "unexpected docker command: $*" >&2
        exit 1
        ;;
esac
EOF
chmod +x "$TEST_TMP/lock-fake-bin/docker"

PATH="$TEST_TMP/lock-fake-bin:$PATH" \
FAKE_LOCK_STARTED="$TEST_TMP/lock-owner-started" \
FAKE_LOCK_RELEASE="$TEST_TMP/release-lock-owner" \
APP_RUNTIME_ROOT="$TEST_TMP/lock-runtime-a" \
    "$DEV_SCRIPT" up lock-owner >"$TEST_TMP/lock-owner.log" 2>&1 &
LOCK_OWNER_PID=$!

for _ in {1..30}; do
    [[ -f "$TEST_TMP/lock-owner-started" ]] && break
    sleep 1
done
if [[ ! -f "$TEST_TMP/lock-owner-started" ]]; then
    fail "lock owner reaches container creation"
elif PATH="$TEST_TMP/lock-fake-bin:$PATH" \
    FAKE_LOCK_STARTED="$TEST_TMP/lock-contender-started" \
    FAKE_LOCK_RELEASE="$TEST_TMP/release-lock-contender" \
    APP_RUNTIME_ROOT="$TEST_TMP/lock-runtime-b" \
    "$DEV_SCRIPT" up lock-contender >"$TEST_TMP/lock-contender.log" 2>&1; then
    fail "different runtime roots cannot bypass the same-worktree lock"
elif grep -Fq "already used by active instance 'app-lock-owner'" \
    "$TEST_TMP/lock-contender.log"; then
    pass "different runtime roots cannot bypass the same-worktree lock"
else
    fail "different runtime roots cannot bypass the same-worktree lock"
fi
touch "$TEST_TMP/release-lock-owner"
wait "$LOCK_OWNER_PID" 2>/dev/null || true
LOCK_OWNER_PID=""

mkdir -p "$TEST_TMP/runtime"
touch "$TEST_TMP/runtime/instance.env"
chmod 600 "$TEST_TMP/runtime/instance.env"

APP_INSTANCE_ID=app-config-test \
APP_PROJECT_NAME=app-config-test \
APP_RUNTIME_DIR="$TEST_TMP/runtime" \
APP_WORKTREE="$REPO_ROOT" \
    docker compose --project-name app-config-test --file "$COMPOSE_FILE" config \
    >"$TEST_TMP/compose-config.yml"

assert_not_contains \
    "Compose does not use globally fixed container names" \
    'container_name:' \
    "$TEST_TMP/compose-config.yml"
assert_contains "Compose includes the development app" '^  app:$' "$TEST_TMP/compose-config.yml"
assert_contains "Compose includes schema initialization" '^  schema-init:$' "$TEST_TMP/compose-config.yml"
assert_contains "Compose includes PostgreSQL" '^  postgres:$' "$TEST_TMP/compose-config.yml"
assert_contains "Compose includes MinIO" '^  minio:$' "$TEST_TMP/compose-config.yml"
assert_contains "Compose includes Mock OAuth" '^  mock-oauth:$' "$TEST_TMP/compose-config.yml"
assert_contains "Compose includes the GitHub API mock" '^  mock-github-api:$' "$TEST_TMP/compose-config.yml"
assert_contains \
    "published ports bind only to loopback" \
    'host_ip: 127\.0\.0\.1' \
    "$TEST_TMP/compose-config.yml"
assert_contains \
    "Docker assigns published ports dynamically" \
    'published: "0"' \
    "$TEST_TMP/compose-config.yml"
assert_not_contains \
    "Compose does not publish the old fixed development ports" \
    'published: "(5432|8080|8082|9000|9001)"' \
    "$TEST_TMP/compose-config.yml"
assert_contains \
    "Cargo build output has a project-scoped volume" \
    'source: cargo-target' \
    "$TEST_TMP/compose-config.yml"
assert_contains \
    "Dioxus session data has a project-scoped volume" \
    'source: dioxus-cache' \
    "$TEST_TMP/compose-config.yml"
assert_contains \
    "PostgreSQL data has a project-scoped volume" \
    'source: postgres-data' \
    "$TEST_TMP/compose-config.yml"
assert_contains \
    "MinIO data has a project-scoped volume" \
    'source: minio-data' \
    "$TEST_TMP/compose-config.yml"
assert_contains \
    "Cargo defaults to eight parallel build jobs" \
    'CARGO_BUILD_JOBS: "8"' \
    "$TEST_TMP/compose-config.yml"

just --justfile "$REPO_ROOT/justfile" --list >"$TEST_TMP/just-recipes.txt"
assert_contains "just up accepts an optional instance" '^    up instance=' "$TEST_TMP/just-recipes.txt"
assert_contains "just down requires an instance" '^    down instance([[:space:]]|$)' "$TEST_TMP/just-recipes.txt"
assert_contains "instance status is targeted" '^    dev-status instance([[:space:]]|$)' "$TEST_TMP/just-recipes.txt"
assert_contains "instance logs are targeted" '^    dev-logs instance([[:space:]]|$)' "$TEST_TMP/just-recipes.txt"
assert_contains "active instances can be listed" '^    dev-instances([[:space:]]|$)' "$TEST_TMP/just-recipes.txt"
assert_contains \
    "instance listing uses Docker's version-compatible label accessor" \
    '--format.*\{\{\.Label "com\.app\.dev-instance"\}\}' \
    "$DEV_SCRIPT"
assert_contains \
    "worktree locks use a host-user-global root" \
    '^readonly LOCK_ROOT="/tmp/app-dev-locks-\$\{UID\}"$' \
    "$DEV_SCRIPT"
assert_not_contains \
    "worktree locks cannot be bypassed with a different runtime root" \
    'RUNTIME_ROOT/locks' \
    "$DEV_SCRIPT"
assert_contains "host-mode startup has a focused recipe" '^    host-up instance ' "$TEST_TMP/just-recipes.txt"
assert_contains "setup guide documents separate worktrees" 'separate Git worktree' "$REPO_ROOT/docs/new-app-setup.md"
assert_contains "setup guide documents named startup" 'just up instance=my-app-dev' "$REPO_ROOT/docs/new-app-setup.md"
assert_contains "setup guide documents targeted teardown" 'just down instance=my-app-dev' "$REPO_ROOT/docs/new-app-setup.md"

echo
echo "Results: $PASS passed, $FAIL failed"
if [[ "$FAIL" -ne 0 ]]; then
    exit 1
fi
