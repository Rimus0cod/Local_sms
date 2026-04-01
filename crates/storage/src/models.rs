use localmessenger_core::{Device, DeviceId, MemberId};
use localmessenger_crypto::{
    IdentityKeyMaterial, IdentityKeyPair, LocalPrekeyStore, PrekeyStoreMaterial,
};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::StorageError;

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct StorageKey([u8; 32]);

impl StorageKey {
    pub fn generate<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let mut bytes = [0_u8; 32];
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, StorageError> {
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| StorageError::InvalidStorageKeyLength(bytes.len()))?;
        Ok(Self(key))
    }

    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredLocalDeviceSecrets {
    pub device: Device,
    pub identity_key_material: IdentityKeyMaterial,
    pub prekey_store_material: PrekeyStoreMaterial,
}

impl StoredLocalDeviceSecrets {
    pub fn from_runtime(
        device: Device,
        identity_keypair: &IdentityKeyPair,
        prekey_store: &LocalPrekeyStore,
    ) -> Result<Self, StorageError> {
        if device.identity_keys() != &identity_keypair.public_keys() {
            return Err(StorageError::LocalDeviceIdentityMismatch);
        }
        if prekey_store.identity() != device.identity_keys() {
            return Err(StorageError::LocalPrekeyIdentityMismatch);
        }

        Ok(Self {
            device,
            identity_key_material: identity_keypair.to_material(),
            prekey_store_material: prekey_store.to_material(),
        })
    }

    pub fn identity_keypair(&self) -> IdentityKeyPair {
        IdentityKeyPair::from_material(&self.identity_key_material)
    }

    pub fn prekey_store(&self) -> Result<LocalPrekeyStore, StorageError> {
        Ok(LocalPrekeyStore::from_material(
            self.prekey_store_material.clone(),
        )?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoredMessageKind {
    Text,
    Attachment,
    VoiceNote,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_member_id: MemberId,
    pub sender_device_id: DeviceId,
    pub sent_at_unix_ms: i64,
    pub kind: StoredMessageKind,
    pub ciphertext: Vec<u8>,
}

impl StoredMessage {
    pub fn new(
        message_id: impl Into<String>,
        conversation_id: impl Into<String>,
        sender_member_id: MemberId,
        sender_device_id: DeviceId,
        sent_at_unix_ms: i64,
        kind: StoredMessageKind,
        ciphertext: Vec<u8>,
    ) -> Result<Self, StorageError> {
        let message_id = message_id.into();
        let conversation_id = conversation_id.into();
        validate_identifier("message_id", &message_id)?;
        validate_identifier("conversation_id", &conversation_id)?;
        if ciphertext.is_empty() {
            return Err(StorageError::EmptyCiphertext);
        }

        Ok(Self {
            message_id,
            conversation_id,
            sender_member_id,
            sender_device_id,
            sent_at_unix_ms,
            kind,
            ciphertext,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPendingOutbound {
    pub peer_device_id: String,
    pub delivery_order: u64,
    pub message_id: String,
    pub conversation_id: String,
    pub sent_at_unix_ms: i64,
    pub kind: StoredMessageKind,
    pub body: Vec<u8>,
    pub attempt_count: u32,
}

impl StoredPendingOutbound {
    pub fn new(
        peer_device_id: impl Into<String>,
        delivery_order: u64,
        message_id: impl Into<String>,
        conversation_id: impl Into<String>,
        sent_at_unix_ms: i64,
        kind: StoredMessageKind,
        body: Vec<u8>,
        attempt_count: u32,
    ) -> Result<Self, StorageError> {
        let peer_device_id = peer_device_id.into();
        let message_id = message_id.into();
        let conversation_id = conversation_id.into();
        validate_identifier("peer_device_id", &peer_device_id)?;
        validate_identifier("message_id", &message_id)?;
        validate_identifier("conversation_id", &conversation_id)?;
        if body.is_empty() {
            return Err(StorageError::EmptyCiphertext);
        }
        Ok(Self {
            peer_device_id,
            delivery_order,
            message_id,
            conversation_id,
            sent_at_unix_ms,
            kind,
            body,
            attempt_count,
        })
    }
}

pub(crate) fn validate_identifier(field: &'static str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }

    if value.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || character == '-'
            || character == '_'
            || character == ':'
    }) {
        Ok(())
    } else {
        Err(StorageError::InvalidIdentifier {
            field,
            value: value.to_string(),
        })
    }
}
