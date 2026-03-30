use localmessenger_core::{Device, DeviceId, MemberId};
use localmessenger_crypto::{IdentityPublicKeys, InitiatorHandshake};
use serde::{Deserialize, Serialize};

use crate::error::MessagingError;

pub(crate) const SECURE_SESSION_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionPeer {
    pub device_id: String,
    pub member_id: String,
    pub device_name: String,
    pub identity_keys: IdentityPublicKeys,
}

impl SessionPeer {
    pub(crate) fn from_device(device: &Device) -> Self {
        Self {
            device_id: device.device_id().to_string(),
            member_id: device.owner_member_id().to_string(),
            device_name: device.device_name().to_string(),
            identity_keys: device.identity_keys().clone(),
        }
    }

    pub(crate) fn try_into_device(self) -> Result<Device, MessagingError> {
        Ok(Device::new(
            DeviceId::new(self.device_id)?,
            MemberId::new(self.member_id)?,
            self.device_name,
            self.identity_keys,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionRequest {
    pub version: u8,
    pub initiator: SessionPeer,
    pub expected_responder: SessionPeer,
    pub expected_responder_transport_certificate_sha256: [u8; 32],
    pub x3dh: InitiatorHandshake,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionResponse {
    pub version: u8,
    pub responder: SessionPeer,
    pub responder_transport_certificate_sha256: [u8; 32],
    pub consumed_one_time_prekey_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionBinding {
    pub version: u8,
    pub initiator: SessionPeer,
    pub responder: SessionPeer,
    pub responder_transport_certificate_sha256: [u8; 32],
}
