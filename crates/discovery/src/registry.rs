use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use localmessenger_core::DeviceId;

use crate::peer::DiscoveredPeer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryChange {
    Added(DiscoveredPeer),
    Updated(DiscoveredPeer),
    Expired(DiscoveredPeer),
    Unchanged,
}

#[derive(Debug, Clone)]
struct PeerEntry {
    peer: DiscoveredPeer,
    last_seen: Instant,
}

#[derive(Debug)]
pub struct PeerRegistry {
    stale_after: Duration,
    peers: BTreeMap<DeviceId, PeerEntry>,
}

impl PeerRegistry {
    pub fn new(stale_after: Duration) -> Self {
        Self {
            stale_after,
            peers: BTreeMap::new(),
        }
    }

    pub fn upsert_at(&mut self, peer: DiscoveredPeer, seen_at: Instant) -> RegistryChange {
        match self.peers.get_mut(&peer.device_id) {
            Some(existing) => {
                if existing.peer == peer {
                    existing.last_seen = seen_at;
                    RegistryChange::Unchanged
                } else {
                    existing.peer = peer.clone();
                    existing.last_seen = seen_at;
                    RegistryChange::Updated(peer)
                }
            }
            None => {
                self.peers.insert(
                    peer.device_id.clone(),
                    PeerEntry {
                        peer: peer.clone(),
                        last_seen: seen_at,
                    },
                );
                RegistryChange::Added(peer)
            }
        }
    }

    pub fn expire_stale_at(&mut self, now: Instant) -> Vec<RegistryChange> {
        let mut expired_ids = Vec::new();
        for (device_id, entry) in &self.peers {
            if now.duration_since(entry.last_seen) >= self.stale_after {
                expired_ids.push(device_id.clone());
            }
        }

        expired_ids
            .into_iter()
            .filter_map(|device_id| {
                self.peers
                    .remove(&device_id)
                    .map(|entry| RegistryChange::Expired(entry.peer))
            })
            .collect()
    }

    pub fn snapshot(&self) -> Vec<DiscoveredPeer> {
        self.peers
            .values()
            .map(|entry| entry.peer.clone())
            .collect()
    }
}
