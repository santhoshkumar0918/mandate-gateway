-- Identity + access: accounts for merchants, buyer agents, and admins.
--
-- A single table covers all three roles (distinguished by `role`). Each
-- account carries a tenant id so merchant/agent data can be scoped by
-- tenant later. Passwords are argon2id hashes — never plaintext, never in
-- git. Sessions are stateless JWTs (no session table); the gateway signs
-- them with JWT_SECRET.

CREATE TABLE accounts (
    account_id   UUID PRIMARY KEY,
    role         TEXT NOT NULL CHECK (role IN ('merchant','agent','admin')),
    email        TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    tenant_id    TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_accounts_tenant ON accounts (tenant_id);
CREATE INDEX idx_accounts_email ON accounts (email);
