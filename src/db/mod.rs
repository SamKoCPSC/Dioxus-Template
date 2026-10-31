//! PostgreSQL pool and reusable identity/session persistence.
#![allow(dead_code)]
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;
#[derive(Debug)]
pub enum DbError {
    MissingUrl,
    Connection(sqlx::Error),
    InvalidConfiguration(DatabaseConfigError),
    Query(sqlx::Error),
    UsernameTaken,
    SessionInvalid,
}
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingUrl => write!(f, "DATABASE_URL not set"),
            Self::Connection(e) | Self::Query(e) => write!(f, "database operation failed: {e}"),
            Self::InvalidConfiguration(e) => write!(f, "invalid database configuration: {e}"),
            Self::UsernameTaken => write!(f, "Username taken"),
            Self::SessionInvalid => write!(f, "Invalid session"),
        }
    }
}
impl std::error::Error for DbError {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfigError {
    variable: &'static str,
    reason: &'static str,
}

impl std::fmt::Display for DatabaseConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} {}", self.variable, self.reason)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatabasePoolConfig {
    max_connections: u32,
    acquire_timeout: std::time::Duration,
    idle_timeout: std::time::Duration,
    statement_timeout: std::time::Duration,
    transaction_timeout: std::time::Duration,
    idle_in_transaction_timeout: std::time::Duration,
}

impl DatabasePoolConfig {
    fn from_environment() -> Result<Self, DatabaseConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup<F>(lookup: F) -> Result<Self, DatabaseConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let max_connections = parse_bounded_u64(&lookup, "DB_MAX_CONNECTIONS", 5, 1, 100)? as u32;
        let acquire_timeout = duration_from_seconds(&lookup, "DB_ACQUIRE_TIMEOUT_SECS", 10, 1, 60)?;
        let idle_timeout = duration_from_seconds(&lookup, "DB_IDLE_TIMEOUT_SECS", 300, 1, 3600)?;
        let statement_timeout =
            duration_from_millis(&lookup, "DB_STATEMENT_TIMEOUT_MS", 15_000, 100, 300_000)?;
        let transaction_timeout =
            duration_from_millis(&lookup, "DB_TRANSACTION_TIMEOUT_MS", 30_000, 100, 600_000)?;
        let idle_in_transaction_timeout = duration_from_millis(
            &lookup,
            "DB_IDLE_TRANSACTION_TIMEOUT_MS",
            30_000,
            100,
            600_000,
        )?;

        if transaction_timeout < statement_timeout {
            return Err(DatabaseConfigError {
                variable: "DB_TRANSACTION_TIMEOUT_MS",
                reason: "must be greater than or equal to DB_STATEMENT_TIMEOUT_MS",
            });
        }
        if idle_in_transaction_timeout < statement_timeout {
            return Err(DatabaseConfigError {
                variable: "DB_IDLE_TRANSACTION_TIMEOUT_MS",
                reason: "must be greater than or equal to DB_STATEMENT_TIMEOUT_MS",
            });
        }

        Ok(Self {
            max_connections,
            acquire_timeout,
            idle_timeout,
            statement_timeout,
            transaction_timeout,
            idle_in_transaction_timeout,
        })
    }
}

fn parse_bounded_u64<F>(
    lookup: &F,
    variable: &'static str,
    default: u64,
    minimum: u64,
    maximum: u64,
) -> Result<u64, DatabaseConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    let Some(raw) = lookup(variable) else {
        return Ok(default);
    };
    let value = raw.parse::<u64>().map_err(|_| DatabaseConfigError {
        variable,
        reason: "must be an integer",
    })?;
    if !(minimum..=maximum).contains(&value) {
        return Err(DatabaseConfigError {
            variable,
            reason: "is outside the supported safety range",
        });
    }
    Ok(value)
}

fn duration_from_seconds<F>(
    lookup: &F,
    variable: &'static str,
    default: u64,
    minimum: u64,
    maximum: u64,
) -> Result<std::time::Duration, DatabaseConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    parse_bounded_u64(lookup, variable, default, minimum, maximum)
        .map(std::time::Duration::from_secs)
}

fn duration_from_millis<F>(
    lookup: &F,
    variable: &'static str,
    default: u64,
    minimum: u64,
    maximum: u64,
) -> Result<std::time::Duration, DatabaseConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    parse_bounded_u64(lookup, variable, default, minimum, maximum)
        .map(std::time::Duration::from_millis)
}

async fn apply_database_session_limits(
    connection: &mut sqlx::PgConnection,
    config: &DatabasePoolConfig,
) -> Result<(), sqlx::Error> {
    let statement_timeout = format!("{}ms", config.statement_timeout.as_millis());
    let transaction_timeout = format!("{}ms", config.transaction_timeout.as_millis());
    let idle_transaction_timeout = format!("{}ms", config.idle_in_transaction_timeout.as_millis());
    sqlx::query(
        "SELECT set_config('statement_timeout', $1, false), \
                set_config('transaction_timeout', $2, false), \
                set_config('idle_in_transaction_session_timeout', $3, false)",
    )
    .bind(statement_timeout)
    .bind(transaction_timeout)
    .bind(idle_transaction_timeout)
    .execute(connection)
    .await?;
    Ok(())
}

/// Create a new connection pool from the `DATABASE_URL` environment variable.
///
/// Validates connectivity — returns an error if the database is unreachable.
/// The caller is responsible for storing the returned pool (e.g., in Axum state).
///
/// Pool sizing defaults are tuned for a 2-CPU database:
/// - `max_connections`: 5 (formula: cores × 2 + 1; more connections increase
///   context-switching overhead without improving throughput on few cores)
/// - `min_connections`: 1 (avoids cold-start latency on first request)
/// - `idle_timeout`: 5min (releases unused connections promptly)
///
/// Every pooled connection also receives bounded statement, transaction, and
/// idle-in-transaction budgets. All pool values have narrow environment
/// overrides that fail closed on malformed or unsafe values.
pub async fn create_pool() -> Result<PgPool, DbError> {
    let url = std::env::var("DATABASE_URL").map_err(|_| DbError::MissingUrl)?;
    let config = DatabasePoolConfig::from_environment().map_err(DbError::InvalidConfiguration)?;
    let connection_config = config.clone();

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(1)
        .acquire_timeout(config.acquire_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(std::time::Duration::from_secs(1800))
        .after_connect(move |connection, _metadata| {
            let config = connection_config.clone();
            Box::pin(async move { apply_database_session_limits(connection, &config).await })
        })
        .connect(&url)
        .await
        .map_err(DbError::Connection)
}

/// Global pool instance, initialized lazily on first access.
///
/// Used by server functions to access the database without relying on
/// axum request extensions (which don't propagate to FullstackContext).
static POOL: tokio::sync::OnceCell<PgPool> = tokio::sync::OnceCell::const_new();

/// Initialize the global pool. Call once during application startup.
pub async fn init_pool() {
    POOL.set(create_pool().await.expect("Failed to create database pool"))
        .expect("Pool already initialized");
    eprintln!("Database pool initialized");
}

/// Get a clone of the global database pool.
///
/// Panics if the pool has not been initialized via [`init_pool`].
pub fn get_pool() -> PgPool {
    POOL.get()
        .expect("Database pool not initialized. Call db::init_pool() first.")
        .clone()
}

/// A user of the application.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An OAuth provider account linked to a user.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OauthAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub profile_data: Option<serde_json::Value>,
    pub refresh_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

/// Short-lived state for OAuth CSRF protection.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthState {
    pub id: String,
    pub redirect_uri: String,
    pub provider: String,
    pub code_verifier: Option<String>,
    pub purpose: String,
    pub user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// A server-side session row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub refreshed_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

/// Display-oriented row for listing OAuth accounts on the settings page.
/// Omits `email_verified` and `profile_data` which are not needed for display.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OauthAccountRow {
    pub id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

// ── Auth state queries ──────────────────────────────────────────────────────

/// Insert an auth state for OAuth CSRF protection.
#[cfg(test)]
pub async fn insert_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
    provider: &str,
    redirect_uri: &str,
    code_verifier: &str,
    purpose: &str,
    user_id: Option<Uuid>,
) -> Result<(), DbError> {
    // This compatibility helper is retained for low-level state tests. Real
    // OAuth starts use `insert_bound_auth_state` with a random digest.
    sqlx::query("INSERT INTO auth_states (id, provider, redirect_uri, code_verifier, purpose, user_id, browser_binding_digest) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(id)
        .bind(provider)
        .bind(redirect_uri)
        .bind(code_verifier)
        .bind(purpose)
        .bind(user_id)
        .bind([0_u8; 32].as_slice())
        .execute(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

/// Insert an OAuth state bound to a digest of a browser-held transaction secret.
///
/// The raw secret must never cross this database boundary.
#[allow(clippy::too_many_arguments)]
pub async fn insert_bound_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
    provider: &str,
    redirect_uri: &str,
    code_verifier: &str,
    purpose: &str,
    user_id: Option<Uuid>,
    browser_binding_digest: &[u8],
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO auth_states \
         (id, provider, redirect_uri, code_verifier, purpose, user_id, browser_binding_digest) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(provider)
    .bind(redirect_uri)
    .bind(code_verifier)
    .bind(purpose)
    .bind(user_id)
    .bind(browser_binding_digest)
    .execute(executor)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}

/// Result of attempting to consume a browser-bound OAuth state.
pub enum BoundAuthStateConsume {
    Consumed(AuthState),
    StateNotFound,
    BindingMismatch,
}

/// Consume an OAuth state only when its browser-binding digest matches.
///
/// Locking the row serializes competing callbacks. A mismatch commits no
/// mutation, while the matching callback deletes and returns the row in the
/// same transaction.
pub async fn consume_bound_auth_state(
    pool: &PgPool,
    id: &str,
    browser_binding_digest: &[u8],
) -> Result<BoundAuthStateConsume, DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    let stored: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT browser_binding_digest FROM auth_states WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(DbError::Query)?;

    let Some((stored_digest,)) = stored else {
        tx.commit().await.map_err(DbError::Query)?;
        return Ok(BoundAuthStateConsume::StateNotFound);
    };

    // Both values are SHA-256 digests. Keep comparison time independent of the
    // first differing byte so the browser secret remains an all-or-nothing key.
    let digest_matches = stored_digest.len() == browser_binding_digest.len()
        && stored_digest
            .iter()
            .zip(browser_binding_digest)
            .fold(0_u8, |difference, (stored, supplied)| {
                difference | (stored ^ supplied)
            })
            == 0;
    if !digest_matches {
        tx.commit().await.map_err(DbError::Query)?;
        return Ok(BoundAuthStateConsume::BindingMismatch);
    }

    let auth_state = sqlx::query_as::<_, AuthState>(
        "DELETE FROM auth_states WHERE id = $1 \
         RETURNING id, redirect_uri, provider, code_verifier, purpose, user_id, created_at",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(DbError::Query)?
    .ok_or_else(|| DbError::Query(sqlx::Error::RowNotFound))?;
    tx.commit().await.map_err(DbError::Query)?;
    Ok(BoundAuthStateConsume::Consumed(auth_state))
}

/// Atomically delete an auth state by ID and return the deleted row.
///
/// Uses `DELETE ... RETURNING *` so the first caller consumes the state.
/// Concurrent callers with the same ID will get `None` (row already deleted).
pub async fn delete_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
) -> Result<Option<AuthState>, DbError> {
    sqlx::query_as::<_, AuthState>( "DELETE FROM auth_states WHERE id = $1 RETURNING id, redirect_uri, provider, code_verifier, purpose, user_id, created_at" ).bind(id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Delete all auth states older than 15 minutes.
///
/// Used by the application-level fallback cleanup task when pg_cron is
/// unavailable. Also callable directly for testing.
pub async fn cleanup_expired_auth_states(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
) -> Result<u64, DbError> {
    let result =
        sqlx::query("DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL '15 minutes'")
            .execute(executor)
            .await
            .map_err(DbError::Query)?;
    Ok(result.rows_affected())
}

// ── Session queries ─────────────────────────────────────────────────────────

/// Insert a new session row and return it.
pub async fn insert_session(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<Session, DbError> {
    sqlx::query_as::<_, Session>(
        "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2)
         RETURNING id, user_id, created_at, expires_at, refreshed_at, revoked",
    )
    .bind(user_id)
    .bind(expires_at)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Get an active (non-revoked, non-expired) session by its ID.
///
/// Used by `verify_session` to validate the JWT's `sub` claim against the DB.
/// Returns `None` if the session doesn't exist, is revoked, or is expired.
pub async fn get_active_session(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    session_id: Uuid,
) -> Result<Option<Session>, DbError> {
    sqlx::query_as::<_, Session>(
        "SELECT id, user_id, created_at, expires_at, refreshed_at, revoked
         FROM sessions
         WHERE id = $1 AND revoked = FALSE AND expires_at > NOW()",
    )
    .bind(session_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Lock the user, then the session, until a guarded mutation finishes.
///
/// Return the row without an activity predicate: callers must check expiry
/// against wall-clock time after a lock wait, not the transaction-start time.
/// User-first ordering matches account deletion; KEY SHARE also protects the
/// provider insertion's user FK without blocking ordinary profile updates.
pub async fn lock_session_for_user_mutation(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    session_id: Uuid,
    expected_user_id: Uuid,
) -> Result<Option<Session>, DbError> {
    let user: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE id = $1 FOR KEY SHARE")
        .bind(expected_user_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(DbError::Query)?;
    if user.is_none() {
        return Ok(None);
    }
    sqlx::query_as::<_, Session>(
        "SELECT id, user_id, created_at, expires_at, refreshed_at, revoked
         FROM sessions WHERE id = $1 FOR UPDATE",
    )
    .bind(session_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(DbError::Query)
}

/// Revoke a session by ID. Returns `true` if a row was updated.
pub async fn revoke_session(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    session_id: Uuid,
) -> Result<bool, DbError> {
    let rows = sqlx::query("UPDATE sessions SET revoked = TRUE WHERE id = $1")
        .bind(session_id)
        .execute(executor)
        .await
        .map_err(DbError::Query)?
        .rows_affected();
    Ok(rows > 0)
}

/// Refresh a session: extend `expires_at` and set `refreshed_at`.
///
/// Only refreshes if the session is active (not revoked, not expired).
/// Returns the updated session on success, or `None` if the session is gone.
pub async fn refresh_session(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    session_id: Uuid,
    new_expires_at: DateTime<Utc>,
) -> Result<Option<Session>, DbError> {
    sqlx::query_as::<_, Session>(
        "UPDATE sessions
         SET expires_at = $2, refreshed_at = NOW()
         WHERE id = $1 AND revoked = FALSE AND expires_at > NOW()
         RETURNING id, user_id, created_at, expires_at, refreshed_at, revoked",
    )
    .bind(session_id)
    .bind(new_expires_at)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Delete expired + revoked sessions older than a given age.
///
/// Used by the application maintenance worker (and testable directly).
/// Returns the number of rows deleted.
pub async fn cleanup_expired_sessions(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    older_than: &str, // e.g. "24 hours"
) -> Result<u64, DbError> {
    let result = sqlx::query(
        "DELETE FROM sessions WHERE (revoked = TRUE OR expires_at < NOW()) AND created_at < NOW() - $1::text::interval",
    )
    .bind(older_than)
    .execute(executor)
    .await
    .map_err(DbError::Query)?;
    Ok(result.rows_affected())
}

// ── OAuth account queries ───────────────────────────────────────────────────

/// Get an OAuth account by provider and provider user ID.
pub async fn get_oauth_account_by_provider(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<OauthAccount>, DbError> {
    sqlx::query_as::<_, OauthAccount>(
        "SELECT id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at \
         FROM oauth_accounts \
         WHERE provider = $1 AND provider_user_id = $2",
    )
    .bind(provider)
    .bind(provider_user_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Get an OAuth account by email for database-layer regression tests.
#[cfg(test)]
pub async fn get_oauth_account_by_email(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    email: &str,
) -> Result<Option<OauthAccount>, DbError> {
    sqlx::query_as::<_, OauthAccount>(
        "SELECT id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at \
         FROM oauth_accounts \
         WHERE email = $1 \
         LIMIT 1",
    )
    .bind(email)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Update the `last_used_at` timestamp for an OAuth account.
pub async fn update_oauth_last_used(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
) -> Result<(), DbError> {
    sqlx::query("UPDATE oauth_accounts SET last_used_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

/// Refresh mutable provider metadata for an existing stable identity.
pub async fn update_oauth_last_used_and_email(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
    email: Option<&str>,
    email_verified: bool,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE oauth_accounts \
         SET last_used_at = NOW(), email = $2, email_verified = $3 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(email)
    .bind(email_verified)
    .execute(executor)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}

/// Get all OAuth accounts linked to a user, for display on the settings page.
pub async fn get_oauth_accounts_by_user(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<OauthAccountRow>, DbError> {
    sqlx::query_as::<_, OauthAccountRow>(
        "SELECT id, provider, provider_user_id, email, created_at, last_used_at \
         FROM oauth_accounts \
         WHERE user_id = $1 \
         ORDER BY provider",
    )
    .bind(user_id)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Get all OAuth accounts for a user (full records including refresh tokens).
/// Used by the revocation flow before account deletion.
pub async fn get_oauth_accounts_by_user_id(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<OauthAccount>, DbError> {
    sqlx::query_as::<_, OauthAccount>(
        "SELECT id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at \
         FROM oauth_accounts \
         WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Delete a single OAuth account, guarded by user_id to prevent cross-user deletion.
///
/// Revokes the OAuth token with the provider before deletion. Revocation
/// failures are logged but never block deletion.
pub async fn delete_oauth_account(
    pool: &PgPool,
    account_id: Uuid,
    user_id: Uuid,
) -> Result<(), DbError> {
    // Fetch the account first to get the refresh token for revocation
    let account = sqlx::query_as::<_, OauthAccount>(
        "SELECT id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at \
         FROM oauth_accounts WHERE id = $1 AND user_id = $2",
    )
    .bind(account_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?
    .ok_or_else(|| DbError::Query(sqlx::Error::RowNotFound))?;

    // Revoke the token before deletion (non-blocking — failures are logged)
    crate::auth::revoke::revoke_account(&account).await;

    // Now delete the account
    let rows = sqlx::query("DELETE FROM oauth_accounts WHERE id = $1 AND user_id = $2")
        .bind(account_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Query)?
        .rows_affected();

    if rows == 0 {
        return Err(DbError::Query(sqlx::Error::RowNotFound));
    }
    Ok(())
}

/// Count the number of OAuth accounts linked to a user.
pub async fn count_oauth_accounts(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<i64, DbError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM oauth_accounts WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(count)
}

// ── User queries ────────────────────────────────────────────────────────────

/// Insert a new user. Returns the created user with the generated ID.
pub async fn insert_user(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    username: &str,
    display_name: &str,
    email: &str,
    avatar_url: Option<&str>,
) -> Result<User, DbError> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (username, display_name, email, avatar_url) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, username, display_name, email, avatar_url, bio, created_at, updated_at",
    )
    .bind(username)
    .bind(display_name)
    .bind(email)
    .bind(avatar_url)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Insert a new OAuth account linked to a user.
pub async fn insert_oauth_account(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    provider: &str,
    provider_user_id: &str,
    email: Option<&str>,
    profile_data: Option<&serde_json::Value>,
    refresh_token: Option<&str>,
) -> Result<OauthAccount, DbError> {
    insert_oauth_account_with_email_verification(
        executor,
        user_id,
        provider,
        provider_user_id,
        email,
        false,
        profile_data,
        refresh_token,
    )
    .await
}

/// Insert an OAuth account while preserving provider email verification state.
#[allow(clippy::too_many_arguments)] // Mirrors the OAuth account columns explicitly.
pub async fn insert_oauth_account_with_email_verification(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    provider: &str,
    provider_user_id: &str,
    email: Option<&str>,
    email_verified: bool,
    profile_data: Option<&serde_json::Value>,
    refresh_token: Option<&str>,
) -> Result<OauthAccount, DbError> {
    sqlx::query_as::<_, OauthAccount>( "INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email, email_verified, profile_data, refresh_token) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at" ).bind(user_id).bind(provider).bind(provider_user_id).bind(email).bind(email_verified).bind(profile_data).bind(refresh_token)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Get a user by ID.
pub async fn get_user_by_id(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
) -> Result<Option<User>, DbError> {
    sqlx::query_as::<_, User>("SELECT id, username, display_name, email, avatar_url, bio, created_at, updated_at FROM users WHERE id = $1")
    .bind(id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Check whether a username already exists.
pub async fn username_exists(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    username: &str,
) -> Result<bool, DbError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(username)
        .fetch_one(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(exists)
}

/// Get a user by email address.
pub async fn get_user_by_email(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    email: &str,
) -> Result<Option<User>, DbError> {
    sqlx::query_as::<_, User>("SELECT id, username, display_name, email, avatar_url, bio, created_at, updated_at FROM users WHERE email = $1")
    .bind(email)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

#[cfg(test)]
pub async fn delete_user(pool: &PgPool, id: Uuid) -> Result<(), DbError> {
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

#[cfg(test)]
mod maintenance_tests {
    #[tokio::test]
    async fn maintenance_deletes_only_old_expired_or_revoked_sessions() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let user = super::insert_user(&pool, "cleanup", "Cleanup", "cleanup@example.test", None)
            .await
            .unwrap();
        for (_label, age, expiry, revoked) in [
            ("old-expired", 48, -1, false),
            ("old-revoked", 48, 1, true),
            ("old-active", 48, 1, false),
            ("recent-expired", 1, -1, false),
        ] {
            sqlx::query("INSERT INTO sessions(user_id,created_at,expires_at,revoked) VALUES($1,NOW()-$2*INTERVAL '1 hour',NOW()+$3*INTERVAL '1 hour',$4)")
                .bind(user.id).bind(age as f64).bind(expiry as f64).bind(revoked).execute(&pool).await.unwrap();
        }
        assert_eq!(
            super::cleanup_expired_sessions(&pool, "24 hours")
                .await
                .unwrap(),
            2
        );
        let remaining: (i64, i64) =
            sqlx::query_as("SELECT COUNT(*),COUNT(*) FILTER(WHERE revoked) FROM sessions")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(remaining, (2, 0));
        assert_eq!(super::cleanup_expired_auth_states(&pool).await.unwrap(), 0);
    }
}
