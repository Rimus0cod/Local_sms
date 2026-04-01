use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc};

use crate::registry::RegistryDatabase;
use localmessenger_server_protocol::{
    PeerFrame, PeerQueued, PeerRelayFrame, PeerUnavailable, PeerUnavailableReason, ServerEnvelope,
};

#[derive(Clone)]
pub struct RelayState {
    registry: RegistryDatabase,
    online: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<ServerEnvelope>>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutePeerFrameOutcome {
    Delivered,
    Queued,
}

impl RelayState {
    pub fn new(registry: RegistryDatabase) -> Self {
        Self {
            registry,
            online: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register_online(
        &self,
        device_id: String,
        sender: mpsc::UnboundedSender<ServerEnvelope>,
    ) {
        let mut online = self.online.lock().await;
        if let Some(previous) = online.insert(device_id, sender) {
            let _ = previous.send(ServerEnvelope::Disconnect(
                localmessenger_server_protocol::Disconnect {
                    reason: "superseded by newer session".to_string(),
                },
            ));
        }
    }

    pub async fn unregister_online(&self, device_id: &str) {
        self.online.lock().await.remove(device_id);
    }

    pub async fn online_count(&self) -> usize {
        self.online.lock().await.len()
    }

    pub async fn route_peer_frame(
        &self,
        sender_device_id: &str,
        frame: PeerRelayFrame,
        now_unix_ms: i64,
    ) -> Result<RoutePeerFrameOutcome, PeerUnavailable> {
        let recipient = self
            .registry
            .registered_device(&frame.recipient_device_id)
            .await
            .map_err(|_| PeerUnavailable {
                request_id: frame.request_id,
                recipient_device_id: frame.recipient_device_id.clone(),
                reason: PeerUnavailableReason::UnknownRecipient,
            })?;

        let Some(recipient) = recipient else {
            return Err(PeerUnavailable {
                request_id: frame.request_id,
                recipient_device_id: frame.recipient_device_id,
                reason: PeerUnavailableReason::UnknownRecipient,
            });
        };

        if recipient.disabled {
            return Err(PeerUnavailable {
                request_id: frame.request_id,
                recipient_device_id: frame.recipient_device_id,
                reason: PeerUnavailableReason::Disabled,
            });
        }

        let online = self.online.lock().await;
        let Some(target) = online.get(&recipient.device_id) else {
            drop(online);
            self.registry
                .queue_peer_frame(
                    sender_device_id,
                    &frame.recipient_device_id,
                    &frame.payload,
                    now_unix_ms,
                )
                .await
                .map_err(|_| PeerUnavailable {
                    request_id: frame.request_id,
                    recipient_device_id: frame.recipient_device_id.clone(),
                    reason: PeerUnavailableReason::Offline,
                })?;
            return Ok(RoutePeerFrameOutcome::Queued);
        };

        target
            .send(ServerEnvelope::PeerFrame(PeerFrame {
                sender_device_id: sender_device_id.to_string(),
                payload: frame.payload,
            }))
            .map_err(|_| PeerUnavailable {
                request_id: frame.request_id,
                recipient_device_id: recipient.device_id,
                reason: PeerUnavailableReason::Offline,
            })?;
        Ok(RoutePeerFrameOutcome::Delivered)
    }

    pub async fn queued_envelopes_for_recipient(
        &self,
        recipient_device_id: &str,
    ) -> Result<Vec<(i64, ServerEnvelope)>, String> {
        let queued = self
            .registry
            .queued_peer_frames_for_recipient(recipient_device_id)
            .await?;

        Ok(queued
            .into_iter()
            .map(|record| {
                (
                    record.row_id,
                    ServerEnvelope::PeerFrame(PeerFrame {
                        sender_device_id: record.sender_device_id,
                        payload: record.payload,
                    }),
                )
            })
            .collect())
    }

    pub async fn delete_queued_frame(&self, row_id: i64) -> Result<(), String> {
        self.registry.delete_queued_peer_frame(row_id).await
    }
}

pub fn queued_notice(frame: &PeerRelayFrame, queued_at_unix_ms: i64) -> ServerEnvelope {
    ServerEnvelope::PeerQueued(PeerQueued {
        request_id: frame.request_id,
        recipient_device_id: frame.recipient_device_id.clone(),
        queued_at_unix_ms,
    })
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::{RelayState, RoutePeerFrameOutcome, queued_notice};
    use crate::registry::RegistryDatabase;
    use localmessenger_core::{DeviceId, MemberId};
    use localmessenger_server_protocol::{
        DeviceRegistrationBundle, PeerRelayFrame, PeerUnavailableReason, ServerEnvelope,
    };

    async fn register(db: &RegistryDatabase, member: &str, device: &str) {
        let bundle = DeviceRegistrationBundle::new(
            &MemberId::new(member).expect("member"),
            &DeviceId::new(device).expect("device"),
            format!("{member} device"),
            [1_u8; 32],
        );
        db.register_device(&bundle, 1).await.expect("register");
    }

    #[tokio::test]
    async fn relay_routes_online_queues_offline_and_rejects_invalid_recipients() {
        let db = RegistryDatabase::open(":memory:").await.expect("db");
        register(&db, "alice", "alice-phone").await;
        register(&db, "bob", "bob-phone").await;
        register(&db, "carol", "carol-phone").await;
        let relay = RelayState::new(db.clone());

        let (bob_tx, mut bob_rx) = mpsc::unbounded_channel();
        let (carol_tx, mut carol_rx) = mpsc::unbounded_channel();
        relay.register_online("bob-phone".to_string(), bob_tx).await;
        relay
            .register_online("carol-phone".to_string(), carol_tx)
            .await;

        let delivered = relay
            .route_peer_frame(
                "alice-phone",
                PeerRelayFrame {
                    request_id: 1,
                    recipient_device_id: "bob-phone".to_string(),
                    payload: b"hello".to_vec(),
                },
                100,
            )
            .await
            .expect("bob should be online");
        assert_eq!(delivered, RoutePeerFrameOutcome::Delivered);

        let envelope = bob_rx.recv().await.expect("bob should receive");
        match envelope {
            ServerEnvelope::PeerFrame(frame) => {
                assert_eq!(frame.sender_device_id, "alice-phone");
                assert_eq!(frame.payload, b"hello");
            }
            _ => panic!("unexpected envelope"),
        }
        assert!(carol_rx.try_recv().is_err());

        let unavailable = relay
            .route_peer_frame(
                "alice-phone",
                PeerRelayFrame {
                    request_id: 2,
                    recipient_device_id: "missing-device".to_string(),
                    payload: Vec::new(),
                },
                100,
            )
            .await
            .expect_err("missing recipient should fail");
        assert_eq!(unavailable.reason, PeerUnavailableReason::UnknownRecipient);

        relay.unregister_online("bob-phone").await;
        let queued = relay
            .route_peer_frame(
                "alice-phone",
                PeerRelayFrame {
                    request_id: 3,
                    recipient_device_id: "bob-phone".to_string(),
                    payload: b"queued".to_vec(),
                },
                200,
            )
            .await
            .expect("offline recipient should queue");
        assert_eq!(queued, RoutePeerFrameOutcome::Queued);
        let queued_items = relay
            .queued_envelopes_for_recipient("bob-phone")
            .await
            .expect("queued envelopes");
        assert_eq!(queued_items.len(), 1);
        assert!(matches!(
            queued_notice(
                &PeerRelayFrame {
                    request_id: 3,
                    recipient_device_id: "bob-phone".to_string(),
                    payload: b"queued".to_vec(),
                },
                200
            ),
            ServerEnvelope::PeerQueued(_)
        ));

        db.disable_device("carol-phone").await.expect("disable");
        let disabled = relay
            .route_peer_frame(
                "alice-phone",
                PeerRelayFrame {
                    request_id: 4,
                    recipient_device_id: "carol-phone".to_string(),
                    payload: Vec::new(),
                },
                300,
            )
            .await
            .expect_err("disabled recipient should fail");
        assert_eq!(disabled.reason, PeerUnavailableReason::Disabled);
    }
}
