set shell := ["bash", "-cu"]

# Start a disposable environment; each invocation owns its ports/data/caches.
up instance="":
    @bash scripts/dev-environment.sh up {{quote(instance)}}
down instance:
    @bash scripts/dev-environment.sh down {{quote(instance)}}
dev-status instance:
    @bash scripts/dev-environment.sh status {{quote(instance)}}
dev-logs instance:
    @bash scripts/dev-environment.sh logs {{quote(instance)}}
dev-instances:
    @bash scripts/dev-environment.sh instances
configure name slug:
    python3 scripts/configure.py --name {{quote(name)}} --slug {{quote(slug)}}
fmt:
    cargo fmt --all
check:
    cargo check --locked --target wasm32-unknown-unknown
    cargo check --locked --no-default-features --features server
lint:
    cargo fmt --all --check
    cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings
    cargo clippy --locked --no-default-features --features server -- -D warnings
test:
    cargo test --locked --no-default-features --features server -- --test-threads=4
    python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v
config-test:
    bash scripts/tests/test-isolated-dev-config.sh
    python3 -m unittest discover -s docker/codex-cloud -p 'test_*.py' -v
browser instance:
    python3 scripts/browser-smoke.py --instance {{quote(instance)}}

# Run Rust/Dioxus on the host; Docker supplies only disposable PG/mock providers.
host-up instance:
    python3 scripts/host-dev.py up {{quote(instance)}}
host-down instance:
    python3 scripts/host-dev.py down {{quote(instance)}}
host-status instance:
    python3 scripts/host-dev.py status {{quote(instance)}}
host-browser instance:
    python3 scripts/browser-smoke.py --host --instance {{quote(instance)}}
