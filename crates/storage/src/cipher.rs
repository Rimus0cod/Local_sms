use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use serde::de::DeserializeOwned;
use zeroize::Zeroize;

use crate::error::StorageError;
use crate::models::StorageKey;

const ENCRYPTED_BLOB_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct EncryptedBlob {
    version: u8,
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}

#[derive(Clone)]
pub struct AtRestCipher {
    storage_key: StorageKey,
}

impl AtRestCipher {
    pub fn new(storage_key: StorageKey) -> Self {
        Self { storage_key }
    }

    pub fn encrypt<T>(
        &self,
        namespace: &'static str,
        lookup_key: &[u8],
        value: &T,
    ) -> Result<Vec<u8>, StorageError>
    where
        T: Serialize,
    {
        let mut plaintext = bincode::serialize(value)?;
        let mut nonce = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce);

        let aad = associated_data(namespace, lookup_key);
        let cipher = Aes256Gcm::new_from_slice(self.storage_key.as_bytes())
            .map_err(|_| StorageError::EncryptionFailed)?;
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| StorageError::EncryptionFailed)?;
        plaintext.zeroize();

        bincode::serialize(&EncryptedBlob {
            version: ENCRYPTED_BLOB_VERSION,
            nonce,
            ciphertext,
        })
        .map_err(StorageError::from)
    }

    pub fn decrypt<T>(
        &self,
        namespace: &'static str,
        lookup_key: &[u8],
        blob_bytes: &[u8],
    ) -> Result<T, StorageError>
    where
        T: DeserializeOwned,
    {
        let blob: EncryptedBlob = bincode::deserialize(blob_bytes)?;
        if blob.version != ENCRYPTED_BLOB_VERSION {
            return Err(StorageError::InvalidRecordVersion(blob.version));
        }

        let aad = associated_data(namespace, lookup_key);
        let cipher = Aes256Gcm::new_from_slice(self.storage_key.as_bytes())
            .map_err(|_| StorageError::DecryptionFailed)?;
        let mut plaintext = cipher
            .decrypt(
                Nonce::from_slice(&blob.nonce),
                Payload {
                    msg: &blob.ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| StorageError::DecryptionFailed)?;
        let value = bincode::deserialize(&plaintext)?;
        plaintext.zeroize();
        Ok(value)
    }
}

fn associated_data(namespace: &'static str, lookup_key: &[u8]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(namespace.len() + lookup_key.len() + 32);
    aad.extend_from_slice(b"localmessenger/storage/aad/v1");
    aad.extend_from_slice(namespace.as_bytes());
    aad.extend_from_slice(lookup_key);
    aad
}
