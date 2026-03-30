#![forbid(unsafe_code)]

mod error;
mod peer;
mod registry;
mod service;
mod txt;

pub use error::DiscoveryError;
pub use peer::{DiscoveredPeer, LocalPeerAnnouncement, PeerCapability};
pub use registry::{PeerRegistry, RegistryChange};
pub use service::{DEFAULT_SERVICE_TYPE, DiscoveryConfig, DiscoveryEvent, DiscoveryService};

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::{Duration, Instant};

    use dns_parser::Class;
    use localmessenger_core::{DeviceId, MemberId};
    use mdns::{Record, RecordKind, Response};

    use crate::peer::{DiscoveredPeer, LocalPeerAnnouncement, PeerCapability};
    use crate::registry::{PeerRegistry, RegistryChange};
    use crate::service::peer_from_response;
    use crate::txt::{decode_txt_records, encode_txt_records};

    #[test]
    fn txt_round_trip_preserves_announcement_fields() {
        let announcement = LocalPeerAnnouncement::new(
            MemberId::new("alice").expect("member id should be valid"),
            DeviceId::new("alice-phone").expect("device id should be valid"),
            "Alice Phone",
            7070,
            vec![PeerCapability::MessagingV1, PeerCapability::PresenceV1],
        )
        .expect("announcement should be valid");

        let payload = decode_txt_records(encode_txt_records(&announcement))
            .expect("TXT records should round-trip");

        assert_eq!(payload.member_id, announcement.member_id);
        assert_eq!(payload.device_id, announcement.device_id);
        assert_eq!(payload.device_name, announcement.device_name);
        assert_eq!(payload.port, announcement.port);
        assert_eq!(payload.capabilities, announcement.capabilities);
    }

    #[test]
    fn registry_tracks_updates_and_expiration() {
        let peer = sample_peer("alice-phone", 7070);
        let mut registry = PeerRegistry::new(Duration::from_secs(10));
        let start = Instant::now();

        assert!(matches!(
            registry.upsert_at(peer.clone(), start),
            RegistryChange::Added(_)
        ));
        assert!(matches!(
            registry.upsert_at(peer.clone(), start + Duration::from_secs(1)),
            RegistryChange::Unchanged
        ));

        let mut updated = peer.clone();
        updated.socket_address = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9090));
        assert!(matches!(
            registry.upsert_at(updated, start + Duration::from_secs(2)),
            RegistryChange::Updated(_)
        ));

        let expired = registry.expire_stale_at(start + Duration::from_secs(12));
        assert_eq!(expired.len(), 1);
        assert!(registry.snapshot().is_empty());
    }

    #[test]
    fn mdns_response_is_parsed_into_peer() {
        let response = Response {
            answers: vec![
                Record {
                    name: "_localmsg._udp.local".to_string(),
                    class: Class::IN,
                    ttl: 120,
                    kind: RecordKind::PTR(
                        "Alice Phone-alice-phone._localmsg._udp.local".to_string(),
                    ),
                },
                Record {
                    name: "Alice Phone-alice-phone._localmsg._udp.local".to_string(),
                    class: Class::IN,
                    ttl: 120,
                    kind: RecordKind::SRV {
                        priority: 0,
                        weight: 0,
                        port: 7070,
                        target: "alice-phone.local".to_string(),
                    },
                },
                Record {
                    name: "Alice Phone-alice-phone._localmsg._udp.local".to_string(),
                    class: Class::IN,
                    ttl: 120,
                    kind: RecordKind::TXT(vec![
                        "lmv=1".to_string(),
                        "mid=alice".to_string(),
                        "did=alice-phone".to_string(),
                        "name=Alice Phone".to_string(),
                        "port=7070".to_string(),
                        "caps=messaging-v1,presence-v1".to_string(),
                    ]),
                },
            ],
            nameservers: Vec::new(),
            additional: vec![Record {
                name: "alice-phone.local".to_string(),
                class: Class::IN,
                ttl: 120,
                kind: RecordKind::A(Ipv4Addr::new(192, 168, 1, 50)),
            }],
        };

        let peer = peer_from_response(&response)
            .expect("response parsing should succeed")
            .expect("TXT-backed peer should be created");

        assert_eq!(peer.device_id.as_str(), "alice-phone");
        assert_eq!(peer.member_id.as_str(), "alice");
        assert_eq!(peer.port, 7070);
        assert_eq!(
            peer.socket_address,
            Some(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
                7070
            ))
        );
    }

    fn sample_peer(device_id: &str, port: u16) -> DiscoveredPeer {
        DiscoveredPeer {
            service_instance: format!("{device_id}._localmsg._udp.local"),
            member_id: MemberId::new("alice").expect("member id should be valid"),
            device_id: DeviceId::new(device_id).expect("device id should be valid"),
            device_name: "Alice Phone".to_string(),
            port,
            socket_address: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)),
            hostname: Some(format!("{device_id}.local")),
            capabilities: vec![PeerCapability::MessagingV1, PeerCapability::PresenceV1],
        }
    }
}
