use std::collections::BTreeMap;

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::error::CryptoError;
use crate::kdf::{MessageKeyMaterial, chain_kdf, diffie_hellman, root_kdf};

const MESSAGE_VERSION: u8 = 1;
const MAX_SKIP: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRole {
    Initiator,
    Responder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatchetStateSnapshot {
    role: SessionRole,
    local_ratchet_public: [u8; 32],
    remote_ratchet_public: [u8; 32],
    sending_chain_next_message_number: Option<u32>,
    receiving_chain_next_message_number: Option<u32>,
    skipped_message_key_count: usize,
}

impl RatchetStateSnapshot {
    pub fn role(&self) -> SessionRole {
        self.role
    }

    pub fn local_ratchet_public(&self) -> [u8; 32] {
        self.local_ratchet_public
    }

    pub fn remote_ratchet_public(&self) -> [u8; 32] {
        self.remote_ratchet_public
    }

    pub fn sending_chain_next_message_number(&self) -> Option<u32> {
        self.sending_chain_next_message_number
    }

    pub fn receiving_chain_next_message_number(&self) -> Option<u32> {
        self.receiving_chain_next_message_number
    }

    pub fn skipped_message_key_count(&self) -> usize {
        self.skipped_message_key_count
    }
}

pub struct SessionSeed {
    pub role: SessionRole,
    pub root_key: [u8; 32],
    pub local_ratchet_secret: StaticSecret,
    pub local_ratchet_public: [u8; 32],
    pub remote_ratchet_public: [u8; 32],
    pub sending_chain_key: Option<[u8; 32]>,
    pub receiving_chain_key: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageHeader {
    pub ratchet_public: [u8; 32],
    pub previous_chain_length: u32,
    pub message_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub version: u8,
    pub header: MessageHeader,
    pub ciphertext: Vec<u8>,
}

#[derive(Debug)]
struct ChainState {
    key: [u8; 32],
    next_message_number: u32,
}

impl ChainState {
    fn new(key: [u8; 32]) -> Self {
        Self {
            key,
            next_message_number: 0,
        }
    }

    fn current_number(&self) -> u32 {
        self.next_message_number
    }

    fn advance(&mut self) -> Result<(u32, MessageKeyMaterial), CryptoError> {
        let message_number = self.next_message_number;
        let (next_key, message_key) = chain_kdf(&self.key)?;
        self.key = next_key;
        self.next_message_number = self
            .next_message_number
            .checked_add(1)
            .ok_or(CryptoError::InvalidKeyMaterial("chain counter"))?;
        Ok((message_number, message_key))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SkippedMessageId {
    ratchet_public: [u8; 32],
    message_number: u32,
}

pub struct DoubleRatchet {
    role: SessionRole,
    root_key: [u8; 32],
    local_ratchet_secret: StaticSecret,
    local_ratchet_public: [u8; 32],
    remote_ratchet_public: [u8; 32],
    sending_chain: Option<ChainState>,
    receiving_chain: Option<ChainState>,
    previous_chain_length: u32,
    skipped_message_keys: BTreeMap<SkippedMessageId, MessageKeyMaterial>,
    max_skip: u32,
}

impl DoubleRatchet {
    pub fn from_seed(seed: SessionSeed) -> Self {
        Self {
            role: seed.role,
            root_key: seed.root_key,
            local_ratchet_secret: seed.local_ratchet_secret,
            local_ratchet_public: seed.local_ratchet_public,
            remote_ratchet_public: seed.remote_ratchet_public,
            sending_chain: seed.sending_chain_key.map(ChainState::new),
            receiving_chain: seed.receiving_chain_key.map(ChainState::new),
            previous_chain_length: 0,
            skipped_message_keys: BTreeMap::new(),
            max_skip: MAX_SKIP,
        }
    }

    pub fn role(&self) -> SessionRole {
        self.role
    }

    pub fn state_snapshot(&self) -> RatchetStateSnapshot {
        RatchetStateSnapshot {
            role: self.role,
            local_ratchet_public: self.local_ratchet_public,
            remote_ratchet_public: self.remote_ratchet_public,
            sending_chain_next_message_number: self
                .sending_chain
                .as_ref()
                .map(ChainState::current_number),
            receiving_chain_next_message_number: self
                .receiving_chain
                .as_ref()
                .map(ChainState::current_number),
            skipped_message_key_count: self.skipped_message_keys.len(),
        }
    }

    pub fn encrypt(
        &mut self,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<EncryptedMessage, CryptoError> {
        self.ensure_sending_chain()?;
        let sending_chain = self
            .sending_chain
            .as_mut()
            .ok_or(CryptoError::MissingSendingChain)?;
        let (message_number, message_key) = sending_chain.advance()?;

        let header = MessageHeader {
            ratchet_public: self.local_ratchet_public,
            previous_chain_length: self.previous_chain_length,
            message_number,
        };
        let aad = build_associated_data(&header, associated_data)?;
        let ciphertext = encrypt_aead(&message_key, plaintext, &aad)?;

        Ok(EncryptedMessage {
            version: MESSAGE_VERSION,
            header,
            ciphertext,
        })
    }

    pub fn decrypt(
        &mut self,
        message: &EncryptedMessage,
        associated_data: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if message.version != MESSAGE_VERSION {
            return Err(CryptoError::InvalidHeaderVersion(message.version));
        }

        if let Some(plaintext) = self.try_skipped_message_keys(message, associated_data)? {
            return Ok(plaintext);
        }

        if message.header.ratchet_public != self.remote_ratchet_public {
            self.skip_message_keys(message.header.previous_chain_length)?;
            self.apply_remote_ratchet(message.header.ratchet_public)?;
        }

        self.skip_message_keys(message.header.message_number)?;

        let receiving_chain = self
            .receiving_chain
            .as_mut()
            .ok_or(CryptoError::MissingReceivingChain)?;

        if message.header.message_number < receiving_chain.current_number() {
            return Err(CryptoError::ReplayOrDuplicateMessage(
                message.header.message_number,
            ));
        }

        let (_, message_key) = receiving_chain.advance()?;
        let aad = build_associated_data(&message.header, associated_data)?;
        decrypt_aead(&message_key, &message.ciphertext, &aad)
    }

    fn ensure_sending_chain(&mut self) -> Result<(), CryptoError> {
        if self.sending_chain.is_some() {
            return Ok(());
        }

        let new_secret = StaticSecret::random_from_rng(OsRng);
        let new_public = PublicKey::from(&new_secret).to_bytes();
        let dh_output = diffie_hellman(&new_secret, &self.remote_ratchet_public);
        let (next_root, sending_chain_key) = root_kdf(&self.root_key, &dh_output)?;
        self.root_key = next_root;
        self.local_ratchet_secret = new_secret;
        self.local_ratchet_public = new_public;
        self.sending_chain = Some(ChainState::new(sending_chain_key));
        self.previous_chain_length = 0;
        Ok(())
    }

    fn apply_remote_ratchet(&mut self, remote_ratchet_public: [u8; 32]) -> Result<(), CryptoError> {
        let current_sending_length = self
            .sending_chain
            .as_ref()
            .map(ChainState::current_number)
            .unwrap_or(0);
        self.previous_chain_length = current_sending_length;

        let receive_dh = diffie_hellman(&self.local_ratchet_secret, &remote_ratchet_public);
        let (next_root, receiving_chain_key) = root_kdf(&self.root_key, &receive_dh)?;
        self.root_key = next_root;
        self.remote_ratchet_public = remote_ratchet_public;
        self.receiving_chain = Some(ChainState::new(receiving_chain_key));

        let new_local_secret = StaticSecret::random_from_rng(OsRng);
        let new_local_public = PublicKey::from(&new_local_secret).to_bytes();
        let send_dh = diffie_hellman(&new_local_secret, &self.remote_ratchet_public);
        let (next_root, sending_chain_key) = root_kdf(&self.root_key, &send_dh)?;
        self.root_key = next_root;
        self.local_ratchet_secret = new_local_secret;
        self.local_ratchet_public = new_local_public;
        self.sending_chain = Some(ChainState::new(sending_chain_key));

        Ok(())
    }

    fn skip_message_keys(&mut self, until: u32) -> Result<(), CryptoError> {
        let Some(receiving_chain) = self.receiving_chain.as_mut() else {
            return Ok(());
        };

        let current = receiving_chain.current_number();
        if until < current {
            return Ok(());
        }

        if until - current > self.max_skip {
            return Err(CryptoError::MessageNumberTooFarAhead {
                current,
                requested: until,
                max_skip: self.max_skip,
            });
        }

        while receiving_chain.current_number() < until {
            let (message_number, message_key) = receiving_chain.advance()?;
            self.skipped_message_keys.insert(
                SkippedMessageId {
                    ratchet_public: self.remote_ratchet_public,
                    message_number,
                },
                message_key,
            );
        }

        Ok(())
    }

    fn try_skipped_message_keys(
        &mut self,
        message: &EncryptedMessage,
        associated_data: &[u8],
    ) -> Result<Option<Vec<u8>>, CryptoError> {
        let key = SkippedMessageId {
            ratchet_public: message.header.ratchet_public,
            message_number: message.header.message_number,
        };
        let Some(message_key) = self.skipped_message_keys.remove(&key) else {
            return Ok(None);
        };
        let aad = build_associated_data(&message.header, associated_data)?;
        let plaintext = decrypt_aead(&message_key, &message.ciphertext, &aad)?;
        Ok(Some(plaintext))
    }
}

fn build_associated_data(
    header: &MessageHeader,
    application_associated_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let mut aad = b"localmessenger/double-ratchet/v1".to_vec();
    aad.extend_from_slice(application_associated_data);
    aad.extend_from_slice(&bincode::serialize(header)?);
    Ok(aad)
}

fn encrypt_aead(
    key_material: &MessageKeyMaterial,
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(&key_material.cipher_key)
        .map_err(|_| CryptoError::InvalidKeyMaterial("aes-256-gcm"))?;
    let nonce = Nonce::from_slice(&key_material.nonce);
    cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed)
}

fn decrypt_aead(
    key_material: &MessageKeyMaterial,
    ciphertext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(&key_material.cipher_key)
        .map_err(|_| CryptoError::InvalidKeyMaterial("aes-256-gcm"))?;
    let nonce = Nonce::from_slice(&key_material.nonce);
    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad: associated_data,
            },
        )
        .map_err(|_| CryptoError::DecryptionFailed)
}
