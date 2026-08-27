-- Mandate signing-key persistence at rest.
--
-- The gateway's Ed25519 signing key must survive restarts. Rather than
-- regenerating an ephemeral key on every boot (which invalidates every
-- previously issued mandate), we persist the signing key here, encrypted
-- at rest with a master secret derived from the environment.
--
-- The store holds exactly one active signing key row (`id = 'active'`).
-- `encrypted_key` is `nonce(12) || AES-256-GCM-ciphertext` over the raw
-- 32-byte signing key. `key_id` is the hex of the corresponding verifying
-- (public) key, for human/audit correlation. The master secret that decrypts
-- this never touches the database or git — it is provided via env.

CREATE TABLE keychain (
    id             TEXT PRIMARY KEY,            -- 'active'
    key_id         TEXT NOT NULL,               -- hex of verifying key
    encrypted_key  BYTEA NOT NULL,              -- nonce || ciphertext
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
