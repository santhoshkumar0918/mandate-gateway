-- Scoped API keys for buyer agents (and any programmatic client).
--
-- An agent authenticates with an API key instead of a password/JWT. The raw
-- key is shown once at creation; only its SHA-256 hash is stored. Keys are
-- scoped (mandate:issue / purchase:exec), can be revoked, and are tied to
-- the owning account's tenant so agent actions stay scoped.

CREATE TABLE api_keys (
    key_id       UUID PRIMARY KEY,
    account_id   UUID NOT NULL REFERENCES accounts(account_id) ON DELETE CASCADE,
    key_hash     TEXT NOT NULL,            -- SHA-256 of the raw key
    label        TEXT NOT NULL,
    scopes       JSONB NOT NULL,
    tenant_id    TEXT,
    revoked      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_account ON api_keys (account_id);
CREATE INDEX idx_api_keys_hash ON api_keys (key_hash);
