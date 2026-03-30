use localmessenger_core::{DeviceId, MemberId};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

use crate::error::DiscoveryError;

const TXT_PROTOCOL_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PeerCapability {
    MessagingV1,
    FileTransferV1,
    VoiceNotesV1,
    PresenceV1,
}

impl PeerCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MessagingV1 => "messaging-v1",
            Self::FileTransferV1 => "files-v1",
            Self::VoiceNotesV1 => "voice-v1",
            Self::PresenceV1 => "presence-v1",
        }
    }
}

impl fmt::Display for PeerCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for PeerCapability {
    type Err = DiscoveryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "messaging-v1" => Ok(Self::MessagingV1),
            "files-v1" => Ok(Self::FileTransferV1),
            "voice-v1" => Ok(Self::VoiceNotesV1),
            "presence-v1" => Ok(Self::PresenceV1),
            other => Err(DiscoveryError::InvalidCapability(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalPeerAnnouncement {
    pub member_id: MemberId,
    pub device_id: DeviceId,
    pub device_name: String,
    pub port: u16,
    pub capabilities: Vec<PeerCapability>,
}

impl LocalPeerAnnouncement {
    pub fn new(
        member_id: MemberId,
        device_id: DeviceId,
        device_name: impl Into<String>,
        port: u16,
        mut capabilities: Vec<PeerCapability>,
    ) -> Result<Self, DiscoveryError> {
        let device_name = device_name.into();
        if device_name.trim().is_empty() {
            return Err(DiscoveryError::InvalidTxtRecord(
                "device_name cannot be empty".to_string(),
            ));
        }
        if port == 0 {
            return Err(DiscoveryError::InvalidServicePort(port));
        }

        capabilities.sort();
        capabilities.dedup();

        Ok(Self {
            member_id,
            device_id,
            device_name,
            port,
            capabilities,
        })
    }

    pub fn instance_name(&self) -> String {
        format!("{}-{}", self.device_name, self.device_id.as_str())
    }

    pub fn txt_records(&self) -> Vec<String> {
        let caps = self
            .capabilities
            .iter()
            .map(PeerCapability::as_str)
            .collect::<Vec<_>>()
            .join(",");

        vec![
            format!("lmv={TXT_PROTOCOL_VERSION}"),
            format!("mid={}", self.member_id.as_str()),
            format!("did={}", self.device_id.as_str()),
            format!("name={}", self.device_name),
            format!("port={}", self.port),
            format!("caps={caps}"),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub service_instance: String,
    pub member_id: MemberId,
    pub device_id: DeviceId,
    pub device_name: String,
    pub port: u16,
    pub socket_address: Option<SocketAddr>,
    pub hostname: Option<String>,
    pub capabilities: Vec<PeerCapability>,
}

impl DiscoveredPeer {
    pub fn endpoint(&self) -> Option<SocketAddr> {
        self.socket_address
    }
}
