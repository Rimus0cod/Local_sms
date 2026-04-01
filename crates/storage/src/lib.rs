#![forbid(unsafe_code)]

mod cipher;
mod error;
mod models;
mod store;

pub use error::StorageError;
pub use models::{
    StorageKey, StoredLocalDeviceSecrets, StoredMessage, StoredMessageKind, StoredPendingOutbound,
};
pub use store::SqliteStorage;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use localmessenger_core::{Device, DeviceId, MemberId};
    use localmessenger_crypto::{IdentityKeyPair, LocalPrekeyStore};
    use localmessenger_discovery::{DiscoveredPeer, PeerCapability};
    use rand_core::{OsRng, RngCore};

    use crate::{
        SqliteStorage, StorageError, StorageKey, StoredLocalDeviceSecrets, StoredMessage,
        StoredMessageKind,
    };

    fn make_device(
        member_id: &str,
        device_id: &str,
        device_name: &str,
        identity: &IdentityKeyPair,
    ) -> Device {
        Device::from_identity_keypair(
            DeviceId::new(device_id).expect("device id should be valid"),
            MemberId::new(member_id).expect("member id should be valid"),
            device_name,
            identity,
        )
        .expect("device should be created")
    }

    fn make_peer() -> DiscoveredPeer {
        DiscoveredPeer {
            service_instance: "Bob Laptop-bob-laptop".to_string(),
            member_id: MemberId::new("bob").expect("member id should be valid"),
            device_id: DeviceId::new("bob-laptop").expect("device id should be valid"),
            device_name: "Bob Laptop".to_string(),
            port: 7777,
            socket_address: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7777)),
            hostname: Some("bob.local".to_string()),
            capabilities: vec![PeerCapability::MessagingV1, PeerCapability::PresenceV1],
        }
    }

    #[tokio::test]
    async fn sqlite_storage_persists_encrypted_local_device_state() {
        let mut rng = OsRng;
        let storage = SqliteStorage::open("sqlite::memory:", StorageKey::generate(&mut rng))
            .await
            .expect("storage should open");

        let identity = IdentityKeyPair::generate(&mut rng);
        let device = make_device("alice", "alice-phone", "Alice Phone", &identity);
        let prekeys = LocalPrekeyStore::generate(&mut rng, &identity, 101, 3, 5000);
        let secrets = StoredLocalDeviceSecrets::from_runtime(device.clone(), &identity, &prekeys)
            .expect("runtime secrets should validate");

        storage
            .store_local_device_secrets(&secrets)
            .await
            .expect("local secrets should persist");

        let restored = storage
            .local_device_secrets(device.device_id())
            .await
            .expect("local secrets should load")
            .expect("local secrets should exist");
        let restored_device = storage
            .device(device.device_id())
            .await
            .expect("device load should succeed")
            .expect("device should exist");

        assert_eq!(restored_device, device);
        assert_eq!(restored.device, device);
        assert_eq!(
            restored.identity_keypair().public_keys(),
            identity.public_keys()
        );
        assert_eq!(
            restored
                .prekey_store()
                .expect("prekey store should restore")
                .public_bundle(),
            prekeys.public_bundle()
        );

        let blobs = storage
            .raw_encrypted_blobs("local_device_secrets")
            .await
            .expect("raw blobs should load");
        assert_eq!(blobs.len(), 1);
        assert!(
            !blobs[0]
                .windows(b"Alice Phone".len())
                .any(|w| w == b"Alice Phone")
        );
    }

    #[tokio::test]
    async fn sqlite_storage_tracks_devices_peers_and_messages() {
        let mut rng = OsRng;
        let storage = SqliteStorage::open("sqlite::memory:", StorageKey::generate(&mut rng))
            .await
            .expect("storage should open");

        let identity = IdentityKeyPair::generate(&mut rng);
        let device = make_device("alice", "alice-laptop", "Alice Laptop", &identity);
        storage
            .upsert_device(&device)
            .await
            .expect("device should persist");

        let peer = make_peer();
        storage
            .upsert_peer(&peer)
            .await
            .expect("peer should persist");

        let message_one = StoredMessage::new(
            "msg-1",
            "chat_main",
            MemberId::new("alice").expect("member id should be valid"),
            DeviceId::new("alice-laptop").expect("device id should be valid"),
            1_711_111_111_000,
            StoredMessageKind::Text,
            b"ciphertext-one".to_vec(),
        )
        .expect("message should validate");
        let message_two = StoredMessage::new(
            "msg-2",
            "chat_main",
            MemberId::new("bob").expect("member id should be valid"),
            DeviceId::new("bob-laptop").expect("device id should be valid"),
            1_711_111_112_000,
            StoredMessageKind::Text,
            b"ciphertext-two".to_vec(),
        )
        .expect("message should validate");
        storage
            .append_message(&message_one)
            .await
            .expect("first message should persist");
        storage
            .append_message(&message_two)
            .await
            .expect("second message should persist");

        let devices = storage.list_devices().await.expect("devices should list");
        let peers = storage.list_peers().await.expect("peers should list");
        let messages = storage
            .messages_for_conversation("chat_main")
            .await
            .expect("messages should list");

        assert_eq!(devices, vec![device]);
        assert_eq!(peers, vec![peer]);
        assert_eq!(messages, vec![message_one, message_two]);

        let blobs = storage
            .raw_encrypted_blobs("message_log")
            .await
            .expect("message blobs should load");
        assert_eq!(blobs.len(), 2);
        assert!(
            !blobs[0]
                .windows(b"ciphertext-one".len())
                .any(|w| w == b"ciphertext-one")
        );
    }

    #[tokio::test]
    async fn wrong_storage_key_cannot_decrypt_existing_records() {
        let mut rng = OsRng;
        let original_key = StorageKey::generate(&mut rng);
        let db_path =
            std::env::temp_dir().join(format!("localmessenger-storage-{}.sqlite", rng.next_u64()));
        let database_url = format!("sqlite://{}", db_path.display());
        let _ = fs::remove_file(&db_path);

        let storage = SqliteStorage::open(&database_url, original_key)
            .await
            .expect("storage should open");

        let identity = IdentityKeyPair::generate(&mut rng);
        let device = make_device("alice", "alice-main", "Alice Main", &identity);
        storage
            .upsert_device(&device)
            .await
            .expect("device should persist");
        drop(storage);

        let wrong_key_storage = SqliteStorage::open(&database_url, StorageKey::generate(&mut rng))
            .await
            .expect("second storage should open");
        let error = wrong_key_storage
            .device(device.device_id())
            .await
            .expect_err("wrong key must not decrypt existing device");

        let invalid = StoredMessage::new(
            "bad id!",
            "chat_main",
            MemberId::new("alice").expect("member id should be valid"),
            DeviceId::new("alice-main").expect("device id should be valid"),
            0,
            StoredMessageKind::Text,
            b"ciphertext".to_vec(),
        )
        .expect_err("invalid id should fail");
        assert!(matches!(invalid, StorageError::InvalidIdentifier { .. }));
        assert!(matches!(error, StorageError::DecryptionFailed));
        let _ = fs::remove_file(&db_path);
    }

    #[tokio::test]
    async fn pending_outbound_queue_persists_and_restores_across_open() {
        use crate::StoredPendingOutbound;

        let mut rng = OsRng;
        let storage = SqliteStorage::open("sqlite::memory:", StorageKey::generate(&mut rng))
            .await
            .expect("storage should open");

        let entry = StoredPendingOutbound::new(
            "bob-phone",
            0,
            "msg-pending-1",
            "chat-main",
            1_711_111_200_000,
            StoredMessageKind::Text,
            b"encrypted-body-bytes".to_vec(),
            1,
        )
        .expect("pending entry should validate");

        storage
            .upsert_pending_outbound(&entry)
            .await
            .expect("pending entry should persist");

        let loaded = storage
            .pending_outbound_for_peer("bob-phone")
            .await
            .expect("pending entries should load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].message_id, "msg-pending-1");
        assert_eq!(loaded[0].delivery_order, 0);
        assert_eq!(loaded[0].body, b"encrypted-body-bytes");

        storage
            .remove_pending_outbound("bob-phone", "msg-pending-1")
            .await
            .expect("remove should succeed");
        let after_remove = storage
            .pending_outbound_for_peer("bob-phone")
            .await
            .expect("load after remove should succeed");
        assert!(after_remove.is_empty(), "queue must be empty after removal");
    }
}
