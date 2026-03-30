use std::collections::{BTreeMap, BTreeSet};

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use localmessenger_core::{Device, DeviceId, MemberId};
use localmessenger_crypto::CryptoError;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::MessageKind;
use crate::MessagingError;

const GROUP_SENDER_KEY_VERSION: u8 = 1;
const GROUP_MESSAGE_VERSION: u8 = 1;
const MAX_GROUP_PARTICIPANTS: usize = 8;
const MAX_GROUP_SKIP: u32 = 256;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GroupParticipant {
    member_id: MemberId,
    device_id: DeviceId,
}

impl GroupParticipant {
    pub fn new(member_id: MemberId, device_id: DeviceId) -> Self {
        Self {
            member_id,
            device_id,
        }
    }

    pub fn from_device(device: &Device) -> Self {
        Self::new(device.owner_member_id().clone(), device.device_id().clone())
    }

    pub fn member_id(&self) -> &MemberId {
        &self.member_id
    }

    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupMembership {
    participants: BTreeMap<DeviceId, GroupParticipant>,
}

impl GroupMembership {
    pub fn new(
        participants: impl IntoIterator<Item = GroupParticipant>,
    ) -> Result<Self, MessagingError> {
        let mut membership = Self {
            participants: BTreeMap::new(),
        };

        for participant in participants {
            membership.add_participant(participant)?;
        }

        if membership.participants.is_empty() {
            return Err(MessagingError::EmptyGroupMembership);
        }

        Ok(membership)
    }

    pub fn len(&self) -> usize {
        self.participants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.participants.is_empty()
    }

    pub fn contains(&self, device_id: &DeviceId) -> bool {
        self.participants.contains_key(device_id)
    }

    pub fn participant(&self, device_id: &DeviceId) -> Option<&GroupParticipant> {
        self.participants.get(device_id)
    }

    pub fn participants(&self) -> impl Iterator<Item = &GroupParticipant> {
        self.participants.values()
    }

    pub fn add_participant(&mut self, participant: GroupParticipant) -> Result<(), MessagingError> {
        if self.participants.contains_key(participant.device_id()) {
            return Err(MessagingError::DuplicateGroupParticipant(
                participant.device_id().to_string(),
            ));
        }
        if self.participants.len() >= MAX_GROUP_PARTICIPANTS {
            return Err(MessagingError::GroupMembershipLimitExceeded(
                self.participants.len() + 1,
            ));
        }

        self.participants
            .insert(participant.device_id().clone(), participant);
        Ok(())
    }

    pub fn remove_participant(
        &mut self,
        device_id: &DeviceId,
    ) -> Result<GroupParticipant, MessagingError> {
        self.participants
            .remove(device_id)
            .ok_or_else(|| MessagingError::MissingGroupParticipant(device_id.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupSenderKeyDistribution {
    version: u8,
    group_id: String,
    epoch: u64,
    sender_member_id: MemberId,
    sender_device_id: DeviceId,
    distribution_id: u32,
    chain_key_seed: [u8; 32],
    signing_public_key: [u8; 32],
}

impl GroupSenderKeyDistribution {
    pub fn encode(&self) -> Result<Vec<u8>, MessagingError> {
        Ok(bincode::serialize(self)?)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MessagingError> {
        let distribution: Self = bincode::deserialize(bytes)?;
        if distribution.version != GROUP_SENDER_KEY_VERSION {
            return Err(MessagingError::InvalidSenderKeyDistributionVersion(
                distribution.version,
            ));
        }
        Ok(distribution)
    }

    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn sender_member_id(&self) -> &MemberId {
        &self.sender_member_id
    }

    pub fn sender_device_id(&self) -> &DeviceId {
        &self.sender_device_id
    }

    pub fn distribution_id(&self) -> u32 {
        self.distribution_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GroupMessageHeader {
    version: u8,
    group_id: String,
    epoch: u64,
    sender_member_id: MemberId,
    sender_device_id: DeviceId,
    distribution_id: u32,
    message_id: String,
    message_number: u32,
    sent_at_unix_ms: i64,
    kind: MessageKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupEncryptedMessage {
    header: GroupMessageHeader,
    ciphertext: Vec<u8>,
    signature: Vec<u8>,
}

impl GroupEncryptedMessage {
    pub fn encode(&self) -> Result<Vec<u8>, MessagingError> {
        Ok(bincode::serialize(self)?)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MessagingError> {
        let message: Self = bincode::deserialize(bytes)?;
        if message.header.version != GROUP_MESSAGE_VERSION {
            return Err(MessagingError::InvalidGroupMessageVersion(
                message.header.version,
            ));
        }
        Ok(message)
    }

    pub fn group_id(&self) -> &str {
        &self.header.group_id
    }

    pub fn epoch(&self) -> u64 {
        self.header.epoch
    }

    pub fn sender_member_id(&self) -> &MemberId {
        &self.header.sender_member_id
    }

    pub fn sender_device_id(&self) -> &DeviceId {
        &self.header.sender_device_id
    }

    pub fn message_id(&self) -> &str {
        &self.header.message_id
    }

    pub fn message_number(&self) -> u32 {
        self.header.message_number
    }

    pub fn kind(&self) -> MessageKind {
        self.header.kind
    }

    pub fn sent_at_unix_ms(&self) -> i64 {
        self.header.sent_at_unix_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupDecryptedMessage {
    group_id: String,
    epoch: u64,
    sender_member_id: MemberId,
    sender_device_id: DeviceId,
    message_id: String,
    message_number: u32,
    sent_at_unix_ms: i64,
    kind: MessageKind,
    body: Vec<u8>,
}

impl GroupDecryptedMessage {
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn sender_member_id(&self) -> &MemberId {
        &self.sender_member_id
    }

    pub fn sender_device_id(&self) -> &DeviceId {
        &self.sender_device_id
    }

    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    pub fn message_number(&self) -> u32 {
        self.message_number
    }

    pub fn sent_at_unix_ms(&self) -> i64 {
        self.sent_at_unix_ms
    }

    pub fn kind(&self) -> MessageKind {
        self.kind
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupEpochRotation {
    previous_epoch: u64,
    next_epoch: u64,
    reason: GroupRotationReason,
    membership: GroupMembership,
    local_sender_key_distribution: GroupSenderKeyDistribution,
}

impl GroupEpochRotation {
    pub fn previous_epoch(&self) -> u64 {
        self.previous_epoch
    }

    pub fn next_epoch(&self) -> u64 {
        self.next_epoch
    }

    pub fn reason(&self) -> &GroupRotationReason {
        &self.reason
    }

    pub fn membership(&self) -> &GroupMembership {
        &self.membership
    }

    pub fn local_sender_key_distribution(&self) -> &GroupSenderKeyDistribution {
        &self.local_sender_key_distribution
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupRotationReason {
    MemberAdded { device_id: DeviceId },
    MemberRemoved { device_id: DeviceId },
    DeviceCompromised { device_id: DeviceId },
    ManualForwardSecrecyRefresh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SenderMessageKeyMaterial {
    cipher_key: [u8; 32],
    nonce: [u8; 12],
}

struct LocalSenderKeyState {
    distribution_id: u32,
    chain_key: [u8; 32],
    next_message_number: u32,
    signing_key: SigningKey,
}

impl LocalSenderKeyState {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let mut chain_key = [0_u8; 32];
        rng.fill_bytes(&mut chain_key);

        Self {
            distribution_id: rng.next_u32(),
            chain_key,
            next_message_number: 0,
            signing_key: SigningKey::generate(rng),
        }
    }

    fn distribution(
        &self,
        group_id: &str,
        epoch: u64,
        local_device: &Device,
    ) -> GroupSenderKeyDistribution {
        GroupSenderKeyDistribution {
            version: GROUP_SENDER_KEY_VERSION,
            group_id: group_id.to_string(),
            epoch,
            sender_member_id: local_device.owner_member_id().clone(),
            sender_device_id: local_device.device_id().clone(),
            distribution_id: self.distribution_id,
            chain_key_seed: self.chain_key,
            signing_public_key: self.signing_key.verifying_key().to_bytes(),
        }
    }

    fn encrypt_message(
        &mut self,
        group_id: &str,
        epoch: u64,
        local_device: &Device,
        message_id: String,
        kind: MessageKind,
        sent_at_unix_ms: i64,
        body: Vec<u8>,
    ) -> Result<GroupEncryptedMessage, MessagingError> {
        let (message_number, key_material) = self.advance_message_key()?;
        let header = GroupMessageHeader {
            version: GROUP_MESSAGE_VERSION,
            group_id: group_id.to_string(),
            epoch,
            sender_member_id: local_device.owner_member_id().clone(),
            sender_device_id: local_device.device_id().clone(),
            distribution_id: self.distribution_id,
            message_id,
            message_number,
            sent_at_unix_ms,
            kind,
        };

        let aad = build_group_message_aad(&header)?;
        let ciphertext = encrypt_group_message(&key_material, &body, &aad)?;
        let signature = sign_group_message(&self.signing_key, &aad, &ciphertext)?;

        Ok(GroupEncryptedMessage {
            header,
            ciphertext,
            signature,
        })
    }

    fn advance_message_key(&mut self) -> Result<(u32, SenderMessageKeyMaterial), MessagingError> {
        let message_number = self.next_message_number;
        let (next_chain_key, key_material) = group_chain_kdf(&self.chain_key)?;
        self.chain_key = next_chain_key;
        self.next_message_number = self.next_message_number.checked_add(1).ok_or(
            MessagingError::InvalidGroupKeyMaterial("group sender message counter"),
        )?;
        Ok((message_number, key_material))
    }
}

struct RemoteSenderKeyState {
    distribution_id: u32,
    chain_key_seed: [u8; 32],
    chain_key: [u8; 32],
    next_message_number: u32,
    signing_public_key: VerifyingKey,
    skipped_message_keys: BTreeMap<u32, SenderMessageKeyMaterial>,
    message_id_index: BTreeMap<String, u32>,
    message_number_index: BTreeMap<u32, String>,
    max_skip: u32,
}

impl RemoteSenderKeyState {
    fn from_distribution(
        distribution: &GroupSenderKeyDistribution,
    ) -> Result<Self, MessagingError> {
        let signing_public_key = VerifyingKey::from_bytes(&distribution.signing_public_key)
            .map_err(|_| MessagingError::InvalidGroupKeyMaterial("group sender verifying key"))?;

        Ok(Self {
            distribution_id: distribution.distribution_id,
            chain_key_seed: distribution.chain_key_seed,
            chain_key: distribution.chain_key_seed,
            next_message_number: 0,
            signing_public_key,
            skipped_message_keys: BTreeMap::new(),
            message_id_index: BTreeMap::new(),
            message_number_index: BTreeMap::new(),
            max_skip: MAX_GROUP_SKIP,
        })
    }

    fn matches_distribution(&self, distribution: &GroupSenderKeyDistribution) -> bool {
        self.distribution_id == distribution.distribution_id
            && self.chain_key_seed == distribution.chain_key_seed
            && self.signing_public_key.to_bytes() == distribution.signing_public_key
    }

    fn decrypt_message(
        &mut self,
        envelope: &GroupEncryptedMessage,
    ) -> Result<Vec<u8>, MessagingError> {
        let aad = build_group_message_aad(&envelope.header)?;
        verify_group_message_signature(
            &self.signing_public_key,
            &aad,
            &envelope.ciphertext,
            &envelope.signature,
        )?;
        self.ensure_message_fresh(&envelope.header)?;

        if let Some(plaintext) =
            self.try_skipped_message_key(envelope.header.message_number, envelope, &aad)?
        {
            self.remember_message(&envelope.header);
            return Ok(plaintext);
        }

        self.skip_message_keys(envelope.header.message_number)?;
        if envelope.header.message_number < self.next_message_number {
            return Err(
                CryptoError::ReplayOrDuplicateMessage(envelope.header.message_number).into(),
            );
        }

        let (_, key_material) = self.advance_message_key()?;
        let plaintext = decrypt_group_message(&key_material, &envelope.ciphertext, &aad)?;
        self.remember_message(&envelope.header);
        Ok(plaintext)
    }

    fn ensure_message_fresh(&self, header: &GroupMessageHeader) -> Result<(), MessagingError> {
        if let Some(existing_message_number) = self.message_id_index.get(&header.message_id) {
            if *existing_message_number != header.message_number {
                return Err(MessagingError::MessageIdConflict(header.message_id.clone()));
            }
            return Err(CryptoError::ReplayOrDuplicateMessage(header.message_number).into());
        }

        if let Some(existing_message_id) = self.message_number_index.get(&header.message_number) {
            if existing_message_id != &header.message_id {
                return Err(MessagingError::GroupMessageNumberConflict(
                    header.message_number,
                ));
            }
            return Err(CryptoError::ReplayOrDuplicateMessage(header.message_number).into());
        }

        Ok(())
    }

    fn remember_message(&mut self, header: &GroupMessageHeader) {
        self.message_id_index
            .insert(header.message_id.clone(), header.message_number);
        self.message_number_index
            .insert(header.message_number, header.message_id.clone());
    }

    fn try_skipped_message_key(
        &mut self,
        message_number: u32,
        envelope: &GroupEncryptedMessage,
        aad: &[u8],
    ) -> Result<Option<Vec<u8>>, MessagingError> {
        let Some(key_material) = self.skipped_message_keys.remove(&message_number) else {
            return Ok(None);
        };
        let plaintext = decrypt_group_message(&key_material, &envelope.ciphertext, aad)?;
        Ok(Some(plaintext))
    }

    fn skip_message_keys(&mut self, until: u32) -> Result<(), MessagingError> {
        let current = self.next_message_number;
        if until < current {
            return Ok(());
        }
        if until - current > self.max_skip {
            return Err(CryptoError::MessageNumberTooFarAhead {
                current,
                requested: until,
                max_skip: self.max_skip,
            }
            .into());
        }

        while self.next_message_number < until {
            let (message_number, key_material) = self.advance_message_key()?;
            self.skipped_message_keys
                .insert(message_number, key_material);
        }

        Ok(())
    }

    fn advance_message_key(&mut self) -> Result<(u32, SenderMessageKeyMaterial), MessagingError> {
        let message_number = self.next_message_number;
        let (next_chain_key, key_material) = group_chain_kdf(&self.chain_key)?;
        self.chain_key = next_chain_key;
        self.next_message_number = self.next_message_number.checked_add(1).ok_or(
            MessagingError::InvalidGroupKeyMaterial("group receiver message counter"),
        )?;
        Ok((message_number, key_material))
    }
}

pub struct GroupSession {
    group_id: String,
    epoch: u64,
    membership: GroupMembership,
    local_device: Device,
    local_sender_key: LocalSenderKeyState,
    local_message_ids: BTreeSet<String>,
    remote_sender_keys: BTreeMap<DeviceId, RemoteSenderKeyState>,
}

impl GroupSession {
    pub fn create<R>(
        rng: &mut R,
        group_id: impl Into<String>,
        epoch: u64,
        local_device: Device,
        membership: GroupMembership,
    ) -> Result<Self, MessagingError>
    where
        R: RngCore + CryptoRng,
    {
        let group_id = group_id.into();
        validate_identifier("group_id", &group_id)?;

        let Some(participant) = membership.participant(local_device.device_id()) else {
            return Err(MessagingError::LocalDeviceMissingFromGroup);
        };
        if participant.member_id() != local_device.owner_member_id() {
            return Err(MessagingError::GroupParticipantMismatch(
                "local device member id",
            ));
        }

        Ok(Self {
            group_id,
            epoch,
            membership,
            local_device,
            local_sender_key: LocalSenderKeyState::generate(rng),
            local_message_ids: BTreeSet::new(),
            remote_sender_keys: BTreeMap::new(),
        })
    }

    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn membership(&self) -> &GroupMembership {
        &self.membership
    }

    pub fn local_device(&self) -> &Device {
        &self.local_device
    }

    pub fn sender_key_distribution(&self) -> GroupSenderKeyDistribution {
        self.local_sender_key
            .distribution(&self.group_id, self.epoch, &self.local_device)
    }

    pub fn import_sender_key(
        &mut self,
        distribution: GroupSenderKeyDistribution,
    ) -> Result<(), MessagingError> {
        if distribution.version != GROUP_SENDER_KEY_VERSION {
            return Err(MessagingError::InvalidSenderKeyDistributionVersion(
                distribution.version,
            ));
        }
        validate_identifier("group_id", &distribution.group_id)?;
        if distribution.group_id != self.group_id {
            return Err(MessagingError::GroupIdMismatch {
                expected: self.group_id.clone(),
                received: distribution.group_id,
            });
        }
        if distribution.epoch != self.epoch {
            return Err(MessagingError::GroupEpochMismatch {
                expected: self.epoch,
                received: distribution.epoch,
            });
        }

        let Some(participant) = self.membership.participant(&distribution.sender_device_id) else {
            return Err(MessagingError::MissingGroupParticipant(
                distribution.sender_device_id.to_string(),
            ));
        };
        if participant.member_id() != &distribution.sender_member_id {
            return Err(MessagingError::GroupParticipantMismatch(
                "sender-key member/device pair",
            ));
        }

        if distribution.sender_device_id == *self.local_device.device_id() {
            if distribution.distribution_id == self.local_sender_key.distribution_id {
                return Ok(());
            }
            return Err(MessagingError::SenderKeyDistributionConflict {
                device_id: distribution.sender_device_id.to_string(),
                epoch: distribution.epoch,
            });
        }

        match self.remote_sender_keys.get(&distribution.sender_device_id) {
            Some(existing) if existing.matches_distribution(&distribution) => return Ok(()),
            Some(_) => {
                return Err(MessagingError::SenderKeyDistributionConflict {
                    device_id: distribution.sender_device_id.to_string(),
                    epoch: distribution.epoch,
                });
            }
            None => {}
        }

        self.remote_sender_keys.insert(
            distribution.sender_device_id.clone(),
            RemoteSenderKeyState::from_distribution(&distribution)?,
        );
        Ok(())
    }

    pub fn has_sender_key(&self, device_id: &DeviceId) -> bool {
        self.remote_sender_keys.contains_key(device_id)
    }

    pub fn encrypt_message(
        &mut self,
        message_id: impl Into<String>,
        kind: MessageKind,
        sent_at_unix_ms: i64,
        body: Vec<u8>,
    ) -> Result<GroupEncryptedMessage, MessagingError> {
        let message_id = message_id.into();
        validate_identifier("message_id", &message_id)?;
        if self.local_message_ids.contains(&message_id) {
            return Err(MessagingError::DuplicateGroupMessageId(message_id));
        }

        let encrypted = self.local_sender_key.encrypt_message(
            &self.group_id,
            self.epoch,
            &self.local_device,
            message_id.clone(),
            kind,
            sent_at_unix_ms,
            body,
        )?;
        self.local_message_ids.insert(message_id);
        Ok(encrypted)
    }

    pub fn decrypt_message(
        &mut self,
        envelope: &GroupEncryptedMessage,
    ) -> Result<GroupDecryptedMessage, MessagingError> {
        if envelope.header.version != GROUP_MESSAGE_VERSION {
            return Err(MessagingError::InvalidGroupMessageVersion(
                envelope.header.version,
            ));
        }
        validate_identifier("group_id", &envelope.header.group_id)?;
        validate_identifier("message_id", &envelope.header.message_id)?;

        if envelope.header.group_id != self.group_id {
            return Err(MessagingError::GroupIdMismatch {
                expected: self.group_id.clone(),
                received: envelope.header.group_id.clone(),
            });
        }
        if envelope.header.epoch != self.epoch {
            return Err(MessagingError::GroupEpochMismatch {
                expected: self.epoch,
                received: envelope.header.epoch,
            });
        }

        let Some(participant) = self
            .membership
            .participant(&envelope.header.sender_device_id)
        else {
            return Err(MessagingError::MissingGroupParticipant(
                envelope.header.sender_device_id.to_string(),
            ));
        };
        if participant.member_id() != &envelope.header.sender_member_id {
            return Err(MessagingError::GroupParticipantMismatch(
                "group message member/device pair",
            ));
        }

        let sender_state = self
            .remote_sender_keys
            .get_mut(&envelope.header.sender_device_id)
            .ok_or_else(|| {
                MessagingError::UnknownSenderKey(envelope.header.sender_device_id.to_string())
            })?;
        if sender_state.distribution_id != envelope.header.distribution_id {
            return Err(MessagingError::UnknownSenderKey(
                envelope.header.sender_device_id.to_string(),
            ));
        }

        let body = sender_state.decrypt_message(envelope)?;
        Ok(GroupDecryptedMessage {
            group_id: envelope.header.group_id.clone(),
            epoch: envelope.header.epoch,
            sender_member_id: envelope.header.sender_member_id.clone(),
            sender_device_id: envelope.header.sender_device_id.clone(),
            message_id: envelope.header.message_id.clone(),
            message_number: envelope.header.message_number,
            sent_at_unix_ms: envelope.header.sent_at_unix_ms,
            kind: envelope.header.kind,
            body,
        })
    }

    pub fn rotate_for_member_addition<R>(
        &self,
        rng: &mut R,
        participant: GroupParticipant,
    ) -> Result<(Self, GroupEpochRotation), MessagingError>
    where
        R: RngCore + CryptoRng,
    {
        let mut membership = self.membership.clone();
        let added_device_id = participant.device_id().clone();
        membership.add_participant(participant)?;
        self.rotate_with_membership(
            rng,
            membership,
            GroupRotationReason::MemberAdded {
                device_id: added_device_id,
            },
        )
    }

    pub fn rotate_for_member_removal<R>(
        &self,
        rng: &mut R,
        device_id: &DeviceId,
    ) -> Result<(Self, GroupEpochRotation), MessagingError>
    where
        R: RngCore + CryptoRng,
    {
        let mut membership = self.membership.clone();
        membership.remove_participant(device_id)?;
        self.rotate_with_membership(
            rng,
            membership,
            GroupRotationReason::MemberRemoved {
                device_id: device_id.clone(),
            },
        )
    }

    pub fn rotate_for_device_compromise<R>(
        &self,
        rng: &mut R,
        device_id: &DeviceId,
    ) -> Result<(Self, GroupEpochRotation), MessagingError>
    where
        R: RngCore + CryptoRng,
    {
        let mut membership = self.membership.clone();
        membership.remove_participant(device_id)?;
        self.rotate_with_membership(
            rng,
            membership,
            GroupRotationReason::DeviceCompromised {
                device_id: device_id.clone(),
            },
        )
    }

    pub fn rotate_for_manual_rekey<R>(
        &self,
        rng: &mut R,
    ) -> Result<(Self, GroupEpochRotation), MessagingError>
    where
        R: RngCore + CryptoRng,
    {
        self.rotate_with_membership(
            rng,
            self.membership.clone(),
            GroupRotationReason::ManualForwardSecrecyRefresh,
        )
    }

    fn rotate_with_membership<R>(
        &self,
        rng: &mut R,
        membership: GroupMembership,
        reason: GroupRotationReason,
    ) -> Result<(Self, GroupEpochRotation), MessagingError>
    where
        R: RngCore + CryptoRng,
    {
        let next_epoch = self
            .epoch
            .checked_add(1)
            .ok_or(MessagingError::GroupEpochOverflow)?;
        let next_session = Self::create(
            rng,
            self.group_id.clone(),
            next_epoch,
            self.local_device.clone(),
            membership.clone(),
        )?;
        let rotation = GroupEpochRotation {
            previous_epoch: self.epoch,
            next_epoch,
            reason,
            membership,
            local_sender_key_distribution: next_session.sender_key_distribution(),
        };
        Ok((next_session, rotation))
    }
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), MessagingError> {
    if value.trim().is_empty() {
        return Err(MessagingError::InvalidIdentifier {
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
        Err(MessagingError::InvalidIdentifier {
            field,
            value: value.to_string(),
        })
    }
}

fn group_chain_kdf(
    chain_key: &[u8; 32],
) -> Result<([u8; 32], SenderMessageKeyMaterial), MessagingError> {
    let hkdf = Hkdf::<sha2::Sha256>::new(Some(chain_key), b"step");
    let mut output = [0_u8; 76];
    hkdf.expand(b"localmessenger/group-sender/chain", &mut output)
        .map_err(|_| MessagingError::InvalidGroupKeyMaterial("group sender hkdf"))?;

    let mut next_chain_key = [0_u8; 32];
    let mut cipher_key = [0_u8; 32];
    let mut nonce = [0_u8; 12];
    next_chain_key.copy_from_slice(&output[..32]);
    cipher_key.copy_from_slice(&output[32..64]);
    nonce.copy_from_slice(&output[64..]);

    Ok((
        next_chain_key,
        SenderMessageKeyMaterial { cipher_key, nonce },
    ))
}

fn build_group_message_aad(header: &GroupMessageHeader) -> Result<Vec<u8>, MessagingError> {
    let mut aad = b"localmessenger/group-message/aad/v1".to_vec();
    aad.extend_from_slice(&bincode::serialize(header)?);
    Ok(aad)
}

fn encrypt_group_message(
    key_material: &SenderMessageKeyMaterial,
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, MessagingError> {
    let cipher = Aes256Gcm::new_from_slice(&key_material.cipher_key)
        .map_err(|_| MessagingError::InvalidGroupKeyMaterial("group aes-256-gcm"))?;
    cipher
        .encrypt(
            Nonce::from_slice(&key_material.nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed.into())
}

fn decrypt_group_message(
    key_material: &SenderMessageKeyMaterial,
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, MessagingError> {
    let cipher = Aes256Gcm::new_from_slice(&key_material.cipher_key)
        .map_err(|_| MessagingError::InvalidGroupKeyMaterial("group aes-256-gcm"))?;
    cipher
        .decrypt(
            Nonce::from_slice(&key_material.nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| CryptoError::DecryptionFailed.into())
}

fn sign_group_message(
    signing_key: &SigningKey,
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, MessagingError> {
    let payload = signature_payload(aad, ciphertext)?;
    Ok(signing_key.sign(&payload).to_bytes().to_vec())
}

fn verify_group_message_signature(
    verifying_key: &VerifyingKey,
    aad: &[u8],
    ciphertext: &[u8],
    signature_bytes: &[u8],
) -> Result<(), MessagingError> {
    let signature: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| MessagingError::InvalidGroupSignature)?;
    let signature = Signature::from_bytes(&signature);
    let payload = signature_payload(aad, ciphertext)?;
    verifying_key
        .verify(&payload, &signature)
        .map_err(|_| MessagingError::InvalidGroupSignature)
}

fn signature_payload(aad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, MessagingError> {
    let mut payload = b"localmessenger/group-message/signature/v1".to_vec();
    let aad_len = u32::try_from(aad.len())
        .map_err(|_| MessagingError::Serialization("group aad too large".to_string()))?;
    payload.extend_from_slice(&aad_len.to_be_bytes());
    payload.extend_from_slice(aad);
    payload.extend_from_slice(ciphertext);
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use localmessenger_core::{Device, DeviceId, MemberId};
    use rand_core::OsRng;

    use super::{GroupMembership, GroupParticipant, GroupSession};
    use crate::{MessageKind, MessagingError};
    use localmessenger_crypto::CryptoError;

    #[test]
    fn sender_key_distribution_round_trip_and_out_of_order_decrypt_work() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let bob = make_device("bob", "bob-phone", "Bob Phone", &mut rng);
        let membership = build_group_membership([&alice, &bob]);

        let mut alice_group =
            GroupSession::create(&mut rng, "friends", 1, alice.clone(), membership.clone())
                .expect("alice group session should build");
        let mut bob_group =
            GroupSession::create(&mut rng, "friends", 1, bob, membership).expect("bob group");

        let distribution_bytes = alice_group
            .sender_key_distribution()
            .encode()
            .expect("distribution should encode");
        bob_group
            .import_sender_key(
                super::GroupSenderKeyDistribution::decode(&distribution_bytes)
                    .expect("distribution should decode"),
            )
            .expect("bob should import alice sender key");

        let first = alice_group
            .encrypt_message("group-msg-1", MessageKind::Text, 10, b"first".to_vec())
            .expect("first message should encrypt");
        let second = alice_group
            .encrypt_message("group-msg-2", MessageKind::Text, 11, b"second".to_vec())
            .expect("second message should encrypt");

        let second_plaintext = bob_group
            .decrypt_message(&second)
            .expect("receiver should decrypt skipped group message");
        let first_plaintext = bob_group
            .decrypt_message(&first)
            .expect("receiver should decrypt delayed group message");

        assert_eq!(second_plaintext.body(), b"second");
        assert_eq!(first_plaintext.body(), b"first");
        assert_eq!(
            first_plaintext.sender_member_id(),
            &MemberId::new("alice").unwrap()
        );
    }

    #[test]
    fn rotating_epoch_for_member_addition_requires_new_sender_key_distribution() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let bob = make_device("bob", "bob-phone", "Bob Phone", &mut rng);
        let carol = make_device("carol", "carol-phone", "Carol Phone", &mut rng);

        let membership_v1 = build_group_membership([&alice, &bob]);
        let alice_v1 =
            GroupSession::create(&mut rng, "friends", 1, alice.clone(), membership_v1.clone())
                .expect("alice v1 should build");

        let (mut alice_v2, rotation) = alice_v1
            .rotate_for_member_addition(&mut rng, GroupParticipant::from_device(&carol))
            .expect("rotation should succeed");
        assert_eq!(rotation.previous_epoch(), 1);
        assert_eq!(rotation.next_epoch(), 2);
        assert_eq!(rotation.membership().len(), 3);

        let mut bob_v2 =
            GroupSession::create(&mut rng, "friends", 2, bob, rotation.membership().clone())
                .expect("bob v2 should build");
        let mut carol_v2 =
            GroupSession::create(&mut rng, "friends", 2, carol, rotation.membership().clone())
                .expect("carol v2 should build");

        let epoch_two_message = alice_v2
            .encrypt_message("epoch-2-msg", MessageKind::Text, 20, b"welcome".to_vec())
            .expect("epoch two message should encrypt");

        let bob_error = bob_v2
            .decrypt_message(&epoch_two_message)
            .expect_err("bob should require new sender key");
        assert!(matches!(bob_error, MessagingError::UnknownSenderKey(_)));

        bob_v2
            .import_sender_key(rotation.local_sender_key_distribution().clone())
            .expect("bob should import new sender key");
        carol_v2
            .import_sender_key(rotation.local_sender_key_distribution().clone())
            .expect("carol should import new sender key");

        let bob_plaintext = bob_v2
            .decrypt_message(&epoch_two_message)
            .expect("bob should decrypt epoch two");
        let carol_plaintext = carol_v2
            .decrypt_message(&epoch_two_message)
            .expect("carol should decrypt epoch two");

        assert_eq!(bob_plaintext.body(), b"welcome");
        assert_eq!(carol_plaintext.body(), b"welcome");
    }

    #[test]
    fn rotating_epoch_for_member_removal_blocks_removed_member() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let bob = make_device("bob", "bob-phone", "Bob Phone", &mut rng);
        let carol = make_device("carol", "carol-phone", "Carol Phone", &mut rng);

        let membership_v1 = build_group_membership([&alice, &bob, &carol]);
        let alice_v1 =
            GroupSession::create(&mut rng, "friends", 1, alice.clone(), membership_v1.clone())
                .expect("alice v1 should build");
        let mut carol_v1 =
            GroupSession::create(&mut rng, "friends", 1, carol.clone(), membership_v1.clone())
                .expect("carol v1 should build");
        carol_v1
            .import_sender_key(alice_v1.sender_key_distribution())
            .expect("carol should import alice sender key");

        let (mut alice_v2, rotation) = alice_v1
            .rotate_for_member_removal(&mut rng, carol.device_id())
            .expect("removal rotation should succeed");
        assert_eq!(rotation.membership().len(), 2);

        let mut bob_v2 =
            GroupSession::create(&mut rng, "friends", 2, bob, rotation.membership().clone())
                .expect("bob v2 should build");
        bob_v2
            .import_sender_key(rotation.local_sender_key_distribution().clone())
            .expect("bob should import epoch two sender key");

        let epoch_two_message = alice_v2
            .encrypt_message(
                "epoch-2-after-removal",
                MessageKind::Text,
                30,
                b"private".to_vec(),
            )
            .expect("epoch two message should encrypt");

        let bob_plaintext = bob_v2
            .decrypt_message(&epoch_two_message)
            .expect("bob should decrypt epoch two");
        assert_eq!(bob_plaintext.body(), b"private");

        let carol_error = carol_v1
            .decrypt_message(&epoch_two_message)
            .expect_err("removed member must not decrypt next epoch");
        assert!(matches!(
            carol_error,
            MessagingError::GroupEpochMismatch {
                expected: 1,
                received: 2
            }
        ));
    }

    #[test]
    fn manual_rekey_rotates_epoch_without_membership_change() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let bob = make_device("bob", "bob-phone", "Bob Phone", &mut rng);
        let membership = build_group_membership([&alice, &bob]);

        let alice_v1 =
            GroupSession::create(&mut rng, "friends", 1, alice.clone(), membership.clone())
                .expect("alice v1 should build");

        let (alice_v2, rotation) = alice_v1
            .rotate_for_manual_rekey(&mut rng)
            .expect("manual rekey should succeed");

        assert_eq!(rotation.previous_epoch(), 1);
        assert_eq!(rotation.next_epoch(), 2);
        assert_eq!(rotation.membership(), &membership);
        assert!(matches!(
            rotation.reason(),
            super::GroupRotationReason::ManualForwardSecrecyRefresh
        ));
        assert_eq!(alice_v2.membership(), &membership);
    }

    #[test]
    fn tampered_group_message_signature_is_rejected() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let bob = make_device("bob", "bob-phone", "Bob Phone", &mut rng);
        let membership = build_group_membership([&alice, &bob]);

        let mut alice_group =
            GroupSession::create(&mut rng, "friends", 1, alice, membership.clone())
                .expect("alice group session should build");
        let mut bob_group =
            GroupSession::create(&mut rng, "friends", 1, bob, membership).expect("bob group");
        bob_group
            .import_sender_key(alice_group.sender_key_distribution())
            .expect("bob should import alice sender key");

        let mut encrypted = alice_group
            .encrypt_message("tamper-1", MessageKind::Text, 40, b"hello".to_vec())
            .expect("message should encrypt");
        encrypted.signature[0] ^= 0x55;

        let error = bob_group
            .decrypt_message(&encrypted)
            .expect_err("tampered signature must fail");
        assert!(matches!(error, MessagingError::InvalidGroupSignature));
    }

    #[test]
    fn replayed_group_message_is_rejected() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let bob = make_device("bob", "bob-phone", "Bob Phone", &mut rng);
        let membership = build_group_membership([&alice, &bob]);

        let mut alice_group =
            GroupSession::create(&mut rng, "friends", 1, alice, membership.clone())
                .expect("alice group session should build");
        let mut bob_group =
            GroupSession::create(&mut rng, "friends", 1, bob, membership).expect("bob group");
        bob_group
            .import_sender_key(alice_group.sender_key_distribution())
            .expect("bob should import alice sender key");

        let encrypted = alice_group
            .encrypt_message("replay-1", MessageKind::Text, 40, b"hello".to_vec())
            .expect("message should encrypt");
        bob_group
            .decrypt_message(&encrypted)
            .expect("first decrypt should succeed");

        let error = bob_group
            .decrypt_message(&encrypted)
            .expect_err("replayed group message must fail");
        assert!(matches!(
            error,
            MessagingError::Crypto(CryptoError::ReplayOrDuplicateMessage(0))
        ));
    }

    #[test]
    fn duplicate_group_message_id_is_rejected_within_epoch() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let bob = make_device("bob", "bob-phone", "Bob Phone", &mut rng);
        let membership = build_group_membership([&alice, &bob]);

        let mut alice_group = GroupSession::create(&mut rng, "friends", 1, alice, membership)
            .expect("alice group session should build");

        alice_group
            .encrypt_message("dup-group-id", MessageKind::Text, 50, b"one".to_vec())
            .expect("first group message should encrypt");
        let error = alice_group
            .encrypt_message("dup-group-id", MessageKind::Text, 51, b"two".to_vec())
            .expect_err("duplicate group id must fail");

        assert!(matches!(
            error,
            MessagingError::DuplicateGroupMessageId(message_id) if message_id == "dup-group-id"
        ));
    }

    #[test]
    fn same_epoch_sender_key_replacement_is_rejected() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let bob = make_device("bob", "bob-phone", "Bob Phone", &mut rng);
        let membership = build_group_membership([&alice, &bob]);

        let alice_group_a =
            GroupSession::create(&mut rng, "friends", 1, alice.clone(), membership.clone())
                .expect("alice group A should build");
        let alice_group_b = GroupSession::create(&mut rng, "friends", 1, alice, membership.clone())
            .expect("alice group B should build");
        let mut bob_group =
            GroupSession::create(&mut rng, "friends", 1, bob, membership).expect("bob group");

        bob_group
            .import_sender_key(alice_group_a.sender_key_distribution())
            .expect("initial sender key import should succeed");
        let error = bob_group
            .import_sender_key(alice_group_b.sender_key_distribution())
            .expect_err("same-epoch sender-key replacement must fail");

        assert!(matches!(
            error,
            MessagingError::SenderKeyDistributionConflict { device_id, epoch }
                if device_id == "alice-phone" && epoch == 1
        ));
    }

    #[test]
    fn group_membership_rejects_duplicates_and_size_overflow() {
        let mut rng = OsRng;
        let alice = make_device("alice", "alice-phone", "Alice Phone", &mut rng);
        let duplicate = GroupMembership::new([
            GroupParticipant::from_device(&alice),
            GroupParticipant::from_device(&alice),
        ])
        .expect_err("duplicate device must fail");
        assert!(matches!(
            duplicate,
            MessagingError::DuplicateGroupParticipant(device_id) if device_id == "alice-phone"
        ));

        let overflow = GroupMembership::new((0..9).map(|index| {
            GroupParticipant::new(
                MemberId::new(format!("member-{index}")).expect("member id"),
                DeviceId::new(format!("device-{index}")).expect("device id"),
            )
        }))
        .expect_err("group size limit must be enforced");
        assert!(matches!(
            overflow,
            MessagingError::GroupMembershipLimitExceeded(9)
        ));
    }

    fn build_group_membership<'a>(
        devices: impl IntoIterator<Item = &'a Device>,
    ) -> GroupMembership {
        GroupMembership::new(devices.into_iter().map(GroupParticipant::from_device))
            .expect("membership should build")
    }

    fn make_device(member_id: &str, device_id: &str, device_name: &str, rng: &mut OsRng) -> Device {
        let identity = localmessenger_crypto::IdentityKeyPair::generate(rng);
        Device::from_identity_keypair(
            DeviceId::new(device_id).expect("device id should be valid"),
            MemberId::new(member_id).expect("member id should be valid"),
            device_name,
            &identity,
        )
        .expect("device should be created")
    }
}
