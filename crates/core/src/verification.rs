use localmessenger_crypto::IdentityPublicKeys;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::device::Device;
use crate::error::CoreError;

const QR_PAYLOAD_VERSION: u8 = 1;
const SAFETY_NUMBER_CONTEXT: &[u8] = b"localmessenger/safety-number/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMethod {
    SafetyNumber,
    QrCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Pending,
    Verified { method: VerificationMethod },
}

impl VerificationStatus {
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyNumber {
    digest: [u8; 32],
}

impl SafetyNumber {
    pub fn between(first: &Device, second: &Device) -> Self {
        let (left, right) = canonical_pair(first, second);

        let mut hasher = Sha256::new();
        hasher.update(SAFETY_NUMBER_CONTEXT);
        extend_device_fingerprint(&mut hasher, left);
        extend_device_fingerprint(&mut hasher, right);

        let digest = hasher.finalize();
        let mut bytes = [0_u8; 32];
        bytes.copy_from_slice(&digest);
        Self { digest: bytes }
    }

    pub fn digits(&self) -> String {
        let mut groups = Vec::new();
        for chunk in self.digest[..30].chunks_exact(3) {
            let value =
                (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
            groups.push(format!("{:05}", value % 100_000));
        }
        groups.join(" ")
    }

    pub fn matches(&self, other: &SafetyNumber) -> bool {
        self.digest == other.digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceVerificationQr {
    pub version: u8,
    pub member_id: String,
    pub device_id: String,
    pub device_name: String,
    pub identity_keys: IdentityPublicKeys,
    pub safety_number: SafetyNumber,
}

impl DeviceVerificationQr {
    pub fn from_device(device: &Device, paired_with: Option<&Device>) -> Self {
        Self {
            version: QR_PAYLOAD_VERSION,
            member_id: device.owner_member_id().to_string(),
            device_id: device.device_id().to_string(),
            device_name: device.device_name().to_string(),
            identity_keys: device.identity_keys().clone(),
            safety_number: paired_with
                .map(|peer| SafetyNumber::between(device, peer))
                .unwrap_or_else(|| SafetyNumber::between(device, device)),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, CoreError> {
        Ok(bincode::serialize(self)?)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, CoreError> {
        let payload: Self = bincode::deserialize(bytes)?;
        if payload.version != QR_PAYLOAD_VERSION {
            return Err(CoreError::InvalidQrPayloadVersion(payload.version));
        }
        Ok(payload)
    }
}

fn extend_device_fingerprint(hasher: &mut Sha256, device: &Device) {
    hasher.update(device.owner_member_id().as_str().as_bytes());
    hasher.update([0]);
    hasher.update(device.device_id().as_str().as_bytes());
    hasher.update([0]);
    hasher.update(device.identity_keys().agreement_public_key);
    hasher.update(device.identity_keys().signing_public_key);
}

fn canonical_pair<'a>(first: &'a Device, second: &'a Device) -> (&'a Device, &'a Device) {
    let first_key = (
        first.owner_member_id().as_str(),
        first.device_id().as_str(),
        first.identity_keys().agreement_public_key,
        first.identity_keys().signing_public_key,
    );
    let second_key = (
        second.owner_member_id().as_str(),
        second.device_id().as_str(),
        second.identity_keys().agreement_public_key,
        second.identity_keys().signing_public_key,
    );

    if first_key <= second_key {
        (first, second)
    } else {
        (second, first)
    }
}
