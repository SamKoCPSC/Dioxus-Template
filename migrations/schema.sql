-- Declarative baseline. Production is applied by a schema-owning role.
CREATE TABLE IF NOT EXISTS users (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(), username VARCHAR(30) UNIQUE NOT NULL,
 display_name VARCHAR(100) NOT NULL,email VARCHAR(255) UNIQUE NOT NULL,avatar_url TEXT,bio TEXT,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS oauth_accounts (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 provider VARCHAR(20) NOT NULL CHECK(provider IN ('google','github')),
 provider_user_id VARCHAR(255) NOT NULL,email VARCHAR(255),email_verified BOOLEAN NOT NULL DEFAULT FALSE,
 profile_data JSONB,refresh_token TEXT,created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
 UNIQUE(provider,provider_user_id)
);
CREATE INDEX IF NOT EXISTS idx_oauth_accounts_user ON oauth_accounts(user_id);
CREATE TABLE IF NOT EXISTS auth_states (
 id VARCHAR(64) PRIMARY KEY,redirect_uri TEXT NOT NULL,provider TEXT NOT NULL,code_verifier TEXT,
 browser_binding_digest BYTEA NOT NULL CHECK(octet_length(browser_binding_digest)=32),
 purpose TEXT NOT NULL CHECK(purpose IN ('login','link')),created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),user_id UUID
);
CREATE INDEX IF NOT EXISTS idx_auth_states_cleanup ON auth_states(created_at);
CREATE TABLE IF NOT EXISTS sessions (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),expires_at TIMESTAMPTZ NOT NULL DEFAULT(NOW()+INTERVAL '15 minutes'),
 refreshed_at TIMESTAMPTZ,revoked BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expiry ON sessions(expires_at);
-- Example feature. Replace notes when implementing the new product.
CREATE TABLE IF NOT EXISTS notes (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
 title TEXT NOT NULL CHECK(char_length(title) BETWEEN 1 AND 160),
 body TEXT NOT NULL DEFAULT '' CHECK(char_length(body)<=20000),
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_notes_owner_updated ON notes(user_id,updated_at DESC,id);
