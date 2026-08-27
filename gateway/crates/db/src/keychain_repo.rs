use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::error::DbError;

/// Number of leading bytes reserved for the random AES-GCM nonce.
const NONCE_LEN: usize = 12;

/// Derive the 32-byte AES-256 key from a variable-length master secret.
///
/// We hash the master secret with SHA-256 to get a fixed 256-bit key. This
/// is a simple, auditable KDF for an already-high-entropy secret read from
/// the environment. The derived key never leaves this module.
fn derive_key(master_secret: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(master_secret);
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

/// Encrypt `plaintext` with AES-256-GCM under a key derived from
/// `master_secret`. Returns `nonce || ciphertext + tag`.
///
/// # Security
///
/// Authenticated encryption: the ciphertext is bound to the master secret,
/// so tampering is detected. The plaintext (the signing key) is never
/// returned or logged by this module.
fn encrypt_bytes(master_secret: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let key = derive_key(master_secret);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("AES-256-GCM accepts a 32-byte key");

    let mut nonce_bytes = [0u8; NONCE_LEN];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: b"mandate-gateway-signing-key",
            },
        )
        .expect("AES-GCM encryption of the signing key never fails");

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    out
}

/// Decrypt a `nonce || ciphertext` blob produced by [`encrypt_bytes`].
///
/// Returns `Err(DbError::Crypto)` if the master secret is wrong (the auth
/// tag fails) — the caller must surface this rather than silently continuing
/// with a corrupt key.
fn decrypt_bytes(master_secret: &[u8], blob: &[u8]) -> Result<Vec<u8>, DbError> {
    if blob.len() < NONCE_LEN {
        return Err(DbError::Serialization(
            "keychain blob shorter than its nonce".into(),
        ));
    }
    let key = derive_key(master_secret);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("AES-256-GCM accepts a 32-byte key");

    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    cipher
        .decrypt(
            Nonce::from_slice(nonce_bytes),
            Payload {
                msg: ciphertext,
                aad: b"mandate-gateway-signing-key",
            },
        )
        .map_err(|_| {
            DbError::Crypto("keychain decrypt failed — master secret does not match".into())
        })
}

/// Stores and loads the gateway's single active signing key, encrypted at rest.
#[derive(Clone)]
pub struct KeychainRepo {
    pool: PgPool,
}

impl KeychainRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load the active signing key from the database, creating it if absent.
    ///
    /// When no key row exists yet, `fresh_key_bytes` (a newly generated
    /// signing key) is encrypted and stored, then returned. When a row
    /// already exists, the stored key is decrypted and returned — the newly
    /// generated `fresh_key_bytes` is discarded and never persisted.
    ///
    /// # Invariant
    ///
    /// The returned 32 bytes are the canonical signing key for the lifetime
    /// of the gateway. Restarting must yield the same bytes, so mandates
    /// issued before a restart remain verifiable.
    pub async fn load_or_create(
        &self,
        master_secret: &[u8],
        fresh_key_bytes: [u8; 32],
        key_id: &str,
    ) -> Result<[u8; 32], DbError> {
        let row: Option<(String, Vec<u8>)> = sqlx::query_as(
            "SELECT key_id, encrypted_key FROM keychain WHERE id = 'active'",
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some((_stored_key_id, blob)) = row {
            let plaintext = decrypt_bytes(master_secret, &blob)?;
            let mut key = [0u8; 32];
            if plaintext.len() != 32 {
                return Err(DbError::Serialization(format!(
                    "stored signing key has unexpected length {}",
                    plaintext.len()
                )));
            }
            key.copy_from_slice(&plaintext);
            return Ok(key);
        }

        let blob = encrypt_bytes(master_secret, &fresh_key_bytes);
        sqlx::query(
            "INSERT INTO keychain (id, key_id, encrypted_key) VALUES ('active', $1, $2)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(key_id)
        .bind(&blob)
        .execute(&self.pool)
        .await?;

        Ok(fresh_key_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_then_decrypt_roundtrip() {
        let secret = b"a-correct-battery-horse-staple-secret";
        let key_bytes = [7u8; 32];

        let blob = encrypt_bytes(secret, &key_bytes);
        let decrypted = decrypt_bytes(secret, &blob).unwrap();

        assert_eq!(decrypted.as_slice(), &key_bytes);
    }

    #[test]
    fn wrong_master_secret_fails_decrypt() {
        let blob = encrypt_bytes(b"secret-one", &[7u8; 32]);
        let result = decrypt_bytes(b"secret-two", &blob);
        assert!(result.is_err());
    }

    #[test]
    fn ciphertext_is_not_plaintext() {
        let blob = encrypt_bytes(b"secret", &[7u8; 32]);
        // nonce (12) + ciphertext must be longer than the raw key
        assert!(blob.len() > 32);
        // and must not literally contain the plaintext
        assert!(!blob.windows(32).any(|w| w == [7u8; 32]));
    }
}
