# Dependency audit policy

Run `just audit` with cargo-audit 0.22.2. CI rejects advisories, informational
warnings and yanked dependencies except the exact, expiring entries in
security/rustsec-allowlist.txt. The script rejects malformed, duplicate, expired
or unexplained entries. An exception is not an upstream fix.

The initial audit updated Tokio 1.52.3 to 1.54.1 for RUSTSEC-2026-0386, enumset
1.1.12 to 1.1.15 and spin 0.9.8 to 0.9.9 to replace yanked releases. Retest both
compiler targets and the browser workflow after changing the dependency graph.

Two inherited exceptions expire 2026-11-30:

- **RUSTSEC-2023-0071 (rsa 0.9.10):** Cargo.lock includes SQLx's optional MySQL
  package metadata; this app enables only PostgreSQL. `cargo tree --locked
  --no-default-features --features server -i rsa` and the same command with
  `server,storage-s3` have no active path. The audit script enforces this before
  allowing the exception. Enabling RSA/MySQL requires re-evaluation.
- **RUSTSEC-2026-0253 (lru 0.16.4):** the panic-safety defect requires a stored key
  whose destructor panics during pop, followed by use of the cache after unwind.
  Dioxus 0.7.9's `src/isrg/memory_cache.rs` uses String keys. Optional aws-sdk-s3
  1.142.0's `src/s3_express.rs` uses `CacheKey(String)` with no custom Drop.
  Neither has the required panicking destructor. The app defines no LRU cache
  keys. Patched LRU 0.18.2 is outside those dependencies' current semver ranges.
  Reassess these exact key types when upgrading Dioxus/AWS, and prefer upstream
  upgrades removing the advisory as soon as compatible releases exist.

Do not broaden or renew exceptions merely to obtain a green check. Inspect the
current advisory and active feature graph, document evidence, and use the shortest
reasonable expiry. A new application's feature or custom cache key can invalidate
this reasoning even when its lockfile has not changed.
