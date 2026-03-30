use std::collections::BTreeMap;

use localmessenger_core::{DeviceId, MemberId};

use crate::error::DiscoveryError;
use crate::peer::{DiscoveredPeer, LocalPeerAnnouncement, PeerCapability};

pub fn encode_txt_records(local_peer: &LocalPeerAnnouncement) -> Vec<String> {
    local_peer.txt_records()
}

pub fn decode_txt_records(
    records: impl IntoIterator<Item = String>,
) -> Result<TxtPayload, DiscoveryError> {
    let mut map = BTreeMap::new();

    for record in records {
        let (key, value) = record
            .split_once('=')
            .ok_or_else(|| DiscoveryError::InvalidTxtRecord(record.clone()))?;
        map.insert(key.to_string(), value.to_string());
    }

    let version = map
        .get("lmv")
        .ok_or(DiscoveryError::MissingTxtField("lmv"))?;
    if version != "1" {
        return Err(DiscoveryError::InvalidTxtRecord(format!(
            "unsupported TXT version {version}"
        )));
    }

    let member_id = MemberId::new(
        map.get("mid")
            .ok_or(DiscoveryError::MissingTxtField("mid"))?
            .clone(),
    )?;
    let device_id = DeviceId::new(
        map.get("did")
            .ok_or(DiscoveryError::MissingTxtField("did"))?
            .clone(),
    )?;
    let device_name = map
        .get("name")
        .ok_or(DiscoveryError::MissingTxtField("name"))?
        .clone();
    let port = map
        .get("port")
        .ok_or(DiscoveryError::MissingTxtField("port"))?
        .parse::<u16>()
        .map_err(|_| DiscoveryError::InvalidTxtRecord("port".to_string()))?;
    if port == 0 {
        return Err(DiscoveryError::InvalidServicePort(port));
    }

    let capabilities = map
        .get("caps")
        .ok_or(DiscoveryError::MissingTxtField("caps"))?
        .split(',')
        .filter(|value| !value.is_empty())
        .map(str::parse)
        .collect::<Result<Vec<PeerCapability>, _>>()?;

    Ok(TxtPayload {
        member_id,
        device_id,
        device_name,
        port,
        capabilities,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtPayload {
    pub member_id: MemberId,
    pub device_id: DeviceId,
    pub device_name: String,
    pub port: u16,
    pub capabilities: Vec<PeerCapability>,
}

impl TxtPayload {
    pub fn into_peer(
        self,
        service_instance: String,
        socket_address: Option<std::net::SocketAddr>,
        hostname: Option<String>,
    ) -> DiscoveredPeer {
        DiscoveredPeer {
            service_instance,
            member_id: self.member_id,
            device_id: self.device_id,
            device_name: self.device_name,
            port: self.port,
            socket_address,
            hostname,
            capabilities: self.capabilities,
        }
    }
}
