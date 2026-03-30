use localmessenger_crypto::{IdentityKeyPair, IdentityPublicKeys};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::{DeviceId, MemberId};
use crate::verification::{
    DeviceVerificationQr, SafetyNumber, VerificationMethod, VerificationStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    device_id: DeviceId,
    owner_member_id: MemberId,
    device_name: String,
    identity_keys: IdentityPublicKeys,
    verification_status: VerificationStatus,
}

impl Device {
    pub fn new(
        device_id: DeviceId,
        owner_member_id: MemberId,
        device_name: impl Into<String>,
        identity_keys: IdentityPublicKeys,
    ) -> Result<Self, CoreError> {
        let device_name = device_name.into();
        validate_display_name(&device_name)?;

        Ok(Self {
            device_id,
            owner_member_id,
            device_name,
            identity_keys,
            verification_status: VerificationStatus::Pending,
        })
    }

    pub fn from_identity_keypair(
        device_id: DeviceId,
        owner_member_id: MemberId,
        device_name: impl Into<String>,
        identity_keypair: &IdentityKeyPair,
    ) -> Result<Self, CoreError> {
        Self::new(
            device_id,
            owner_member_id,
            device_name,
            identity_keypair.public_keys(),
        )
    }

    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub fn owner_member_id(&self) -> &MemberId {
        &self.owner_member_id
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn identity_keys(&self) -> &IdentityPublicKeys {
        &self.identity_keys
    }

    pub fn verification_status(&self) -> &VerificationStatus {
        &self.verification_status
    }

    pub fn is_verified(&self) -> bool {
        self.verification_status.is_verified()
    }

    pub fn safety_number_with(&self, other: &Device) -> SafetyNumber {
        SafetyNumber::between(self, other)
    }

    pub fn qr_payload(&self, paired_with: Option<&Device>) -> Result<Vec<u8>, CoreError> {
        DeviceVerificationQr::from_device(self, paired_with).encode()
    }

    pub fn verify_with_safety_number(
        &mut self,
        local_reference_device: &Device,
        presented_safety_number: &SafetyNumber,
    ) -> Result<(), CoreError> {
        let expected = SafetyNumber::between(self, local_reference_device);
        if !expected.matches(presented_safety_number) {
            return Err(CoreError::SafetyNumberMismatch);
        }

        self.verification_status = VerificationStatus::Verified {
            method: VerificationMethod::SafetyNumber,
        };
        Ok(())
    }

    pub fn verify_with_qr_payload(&mut self, payload_bytes: &[u8]) -> Result<(), CoreError> {
        let payload = DeviceVerificationQr::decode(payload_bytes)?;
        self.verify_with_qr(&payload)
    }

    pub fn verify_with_qr(&mut self, payload: &DeviceVerificationQr) -> Result<(), CoreError> {
        if payload.member_id != self.owner_member_id.as_str()
            || payload.device_id != self.device_id.as_str()
            || payload.identity_keys != self.identity_keys
        {
            return Err(CoreError::QrPayloadMismatch);
        }

        self.verification_status = VerificationStatus::Verified {
            method: VerificationMethod::QrCode,
        };
        Ok(())
    }
}

fn validate_display_name(value: &str) -> Result<(), CoreError> {
    if value.trim().is_empty() {
        Err(CoreError::EmptyDisplayName)
    } else {
        Ok(())
    }
}
