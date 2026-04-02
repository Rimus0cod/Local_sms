use std::collections::BTreeMap;
use std::time::Duration;

use localmessenger_core::{Device, DeviceId, MemberId};
use localmessenger_crypto::{IdentityKeyMaterial, IdentityKeyPair, LocalPrekeyStore, PrekeyStoreMaterial};
use localmessenger_messaging::{
    MessageKind, MessagingEngine, RemoteSessionOffer, SecureSession, SessionInitiator,
    SessionResponder,
};
use localmessenger_server_protocol::{
    DeviceContactInvite, decode_contact_invite_device_transport_certificate,
};
use tokio::sync::{mpsc, oneshot};

use crate::relay_client::RelayClient;
use localmessenger_transport::TransportIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerTransportPresence {
    LanOnline,
    RelayOnline,
    OfflineButQueueable,
}

#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    PresenceChanged {
        device_id: String,
        presence: PeerTransportPresence,
    },
    InboundMessage {
        device_id: String,
        conversation_id: String,
        message_id: String,
        body: String,
        sent_at_unix_ms: i64,
    },
}

#[derive(Debug, Clone)]
pub struct RealSendOutcome {
    pub outbound_acknowledged: bool,
    pub forward_secrecy_active: bool,
}

#[derive(Clone)]
struct PeerActorHandle {
    command_tx: mpsc::UnboundedSender<PeerCommand>,
}

enum PeerCommand {
    SendText {
        conversation_id: String,
        message_id: String,
        sent_at_unix_ms: i64,
        body: String,
        response: oneshot::Sender<Result<RealSendOutcome, String>>,
    },
}

struct PendingSendText {
    conversation_id: String,
    message_id: String,
    sent_at_unix_ms: i64,
    body: String,
    response: oneshot::Sender<Result<RealSendOutcome, String>>,
}

pub struct ConnectionManager {
    local_device: Device,
    local_identity_material: IdentityKeyMaterial,
    local_prekey_store_material: PrekeyStoreMaterial,
    local_transport_identity: TransportIdentity,
    relay_client: Option<RelayClient>,
    accepted_invites: BTreeMap<String, DeviceContactInvite>,
    peer_actors: BTreeMap<String, PeerActorHandle>,
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    event_rx: mpsc::UnboundedReceiver<ConnectionEvent>,
}

impl ConnectionManager {
    pub fn new(
        local_device: Device,
        local_identity_material: IdentityKeyMaterial,
        local_prekey_store_material: PrekeyStoreMaterial,
        local_transport_identity: TransportIdentity,
        relay_client: Option<RelayClient>,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            local_device,
            local_identity_material,
            local_prekey_store_material,
            local_transport_identity,
            relay_client,
            accepted_invites: BTreeMap::new(),
            peer_actors: BTreeMap::new(),
            event_tx,
            event_rx,
        }
    }

    pub fn set_relay_client(&mut self, relay_client: Option<RelayClient>) {
        self.relay_client = relay_client;
        if self.relay_client.is_some() {
            let device_ids: Vec<String> = self.accepted_invites.keys().cloned().collect();
            for device_id in device_ids {
                let _ = self.ensure_actor(&device_id);
            }
        }
    }

    pub fn has_contact(&self, device_id: &str) -> bool {
        self.accepted_invites.contains_key(device_id)
    }

    pub fn upsert_contact_invite(&mut self, invite: DeviceContactInvite) -> Result<(), String> {
        invite.validate()?;
        let device_id = invite.device_id.clone();
        self.accepted_invites.insert(device_id.clone(), invite);
        self.ensure_actor(&device_id)?;
        Ok(())
    }

    pub fn drain_events(&mut self) -> Vec<ConnectionEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    pub async fn send_text(
        &mut self,
        device_id: &str,
        conversation_id: String,
        message_id: String,
        sent_at_unix_ms: i64,
        body: String,
    ) -> Result<RealSendOutcome, String> {
        self.ensure_actor(device_id)?;
        let handle = self
            .peer_actors
            .get(device_id)
            .cloned()
            .ok_or_else(|| format!("connection actor for '{device_id}' is unavailable"))?;
        let (tx, rx) = oneshot::channel();
        handle
            .command_tx
            .send(PeerCommand::SendText {
                conversation_id,
                message_id,
                sent_at_unix_ms,
                body,
                response: tx,
            })
            .map_err(|_| format!("connection actor for '{device_id}' is offline"))?;
        rx.await
            .map_err(|_| format!("connection actor for '{device_id}' did not respond"))?
    }

    fn ensure_actor(&mut self, device_id: &str) -> Result<(), String> {
        if self.peer_actors.contains_key(device_id) {
            return Ok(());
        }
        let relay_client = match &self.relay_client {
            Some(client) => client.clone(),
            None => return Ok(()),
        };
        let invite = self
            .accepted_invites
            .get(device_id)
            .cloned()
            .ok_or_else(|| format!("remote contact invite for '{device_id}' is missing"))?;
        let local_device = self.local_device.clone();
        let local_identity_material = self.local_identity_material.clone();
        let local_prekey_store_material = self.local_prekey_store_material.clone();
        let local_transport_identity = self.local_transport_identity.clone();
        let event_tx = self.event_tx.clone();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            run_peer_actor(
                local_device,
                local_identity_material,
                local_prekey_store_material,
                local_transport_identity,
                relay_client,
                invite,
                command_rx,
                event_tx,
            )
            .await;
        });
        self.peer_actors
            .insert(device_id.to_string(), PeerActorHandle { command_tx });
        Ok(())
    }
}

async fn run_peer_actor(
    local_device: Device,
    local_identity_material: IdentityKeyMaterial,
    local_prekey_store_material: PrekeyStoreMaterial,
    local_transport_identity: TransportIdentity,
    relay_client: RelayClient,
    invite: DeviceContactInvite,
    mut command_rx: mpsc::UnboundedReceiver<PeerCommand>,
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,
) {
    let remote_device = match remote_device_from_invite(&invite) {
        Ok(device) => device,
        Err(_) => return,
    };
    let remote_transport_certificate = match decode_contact_invite_device_transport_certificate(&invite)
    {
        Ok(certificate) => certificate,
        Err(_) => return,
    };

    loop {
        let Ok((mut session, mut engine, pending_send)) = establish_or_wait_for_session(
            &local_device,
            &local_identity_material,
            &local_prekey_store_material,
            &local_transport_identity,
            &relay_client,
            &remote_device,
            &invite,
            &remote_transport_certificate,
            &mut command_rx,
        )
        .await
        else {
            break;
        };

        let _ = event_tx.send(ConnectionEvent::PresenceChanged {
            device_id: remote_device.device_id().to_string(),
            presence: PeerTransportPresence::RelayOnline,
        });

        if let Some(pending_send) = pending_send {
            if handle_pending_send(
                &mut session,
                &mut engine,
                &event_tx,
                &remote_device,
                pending_send,
            )
            .await
            .is_err()
            {
                let _ = event_tx.send(ConnectionEvent::PresenceChanged {
                    device_id: remote_device.device_id().to_string(),
                    presence: PeerTransportPresence::OfflineButQueueable,
                });
                continue;
            }
        }

        loop {
            tokio::select! {
                maybe_command = command_rx.recv() => {
                    let Some(command) = maybe_command else {
                        let _ = event_tx.send(ConnectionEvent::PresenceChanged {
                            device_id: remote_device.device_id().to_string(),
                            presence: PeerTransportPresence::OfflineButQueueable,
                        });
                        return;
                    };
                    let pending_send = match command {
                        PeerCommand::SendText {
                            conversation_id,
                            message_id,
                            sent_at_unix_ms,
                            body,
                            response,
                        } => PendingSendText {
                            conversation_id,
                            message_id,
                            sent_at_unix_ms,
                            body,
                            response,
                        },
                    };
                    if handle_pending_send(
                        &mut session,
                        &mut engine,
                        &event_tx,
                        &remote_device,
                        pending_send,
                    ).await.is_err() {
                        let _ = event_tx.send(ConnectionEvent::PresenceChanged {
                            device_id: remote_device.device_id().to_string(),
                            presence: PeerTransportPresence::OfflineButQueueable,
                        });
                        break;
                    }
                }
                inbound = engine.receive_next(&mut session) => {
                    match inbound {
                        Ok(outcome) => emit_delivered_messages(&event_tx, &remote_device, &outcome),
                        Err(_) => {
                            let _ = event_tx.send(ConnectionEvent::PresenceChanged {
                                device_id: remote_device.device_id().to_string(),
                                presence: PeerTransportPresence::OfflineButQueueable,
                            });
                            break;
                        }
                    }
                }
            }
        }
    }
}

async fn establish_or_wait_for_session(
    local_device: &Device,
    local_identity_material: &IdentityKeyMaterial,
    local_prekey_store_material: &PrekeyStoreMaterial,
    local_transport_identity: &TransportIdentity,
    relay_client: &RelayClient,
    remote_device: &Device,
    invite: &DeviceContactInvite,
    remote_transport_certificate: &[u8],
    command_rx: &mut mpsc::UnboundedReceiver<PeerCommand>,
) -> Result<(SecureSession, MessagingEngine, Option<PendingSendText>), String> {
    let remote_offer = RemoteSessionOffer::from_parts(
        remote_device.clone(),
        invite.prekey_bundle.clone(),
        localmessenger_messaging::transport_certificate_sha256(remote_transport_certificate),
    )
    .map_err(|error| error.to_string())?;

    let responder = build_responder(
        local_device,
        local_identity_material,
        local_prekey_store_material,
        local_transport_identity,
    )?;
    let inbound_channel = relay_client.peer_channel(remote_device.device_id()).await;
    let mut accept_future = Box::pin(async move {
        let mut responder = responder;
        responder.accept(inbound_channel).await.map_err(|error| error.to_string())
    });

    tokio::select! {
        accept_result = &mut accept_future => {
            let session = accept_result?;
            let engine = MessagingEngine::from_session(&session);
            Ok((session, engine, None))
        }
        maybe_command = command_rx.recv() => {
            let Some(command) = maybe_command else {
                return Err("peer command channel closed".to_string());
            };
            let pending_send = match command {
                PeerCommand::SendText {
                    conversation_id,
                    message_id,
                    sent_at_unix_ms,
                    body,
                    response,
                } => PendingSendText {
                    conversation_id,
                    message_id,
                    sent_at_unix_ms,
                    body,
                    response,
                },
            };
            let outbound_channel = relay_client.peer_channel(remote_device.device_id()).await;
            let initiator = SessionInitiator::new(
                local_device.clone(),
                IdentityKeyPair::from_material(local_identity_material),
            )
            .map_err(|error| error.to_string())?;
            let session = initiator
                .establish(outbound_channel, &remote_offer, remote_transport_certificate)
                .await
                .map_err(|error| error.to_string())?;
            let engine = MessagingEngine::from_session(&session);
            Ok((session, engine, Some(pending_send)))
        }
    }
}

fn build_responder(
    local_device: &Device,
    local_identity_material: &IdentityKeyMaterial,
    local_prekey_store_material: &PrekeyStoreMaterial,
    local_transport_identity: &TransportIdentity,
) -> Result<SessionResponder, String> {
    let mut sanitized_prekeys = local_prekey_store_material.clone();
    sanitized_prekeys.one_time_prekeys.clear();
    let prekey_store =
        LocalPrekeyStore::from_material(sanitized_prekeys).map_err(|error| error.to_string())?;
    SessionResponder::new(
        local_device.clone(),
        IdentityKeyPair::from_material(local_identity_material),
        prekey_store,
        &local_transport_identity.certificate_der,
    )
    .map_err(|error| error.to_string())
}

async fn handle_pending_send(
    session: &mut SecureSession,
    engine: &mut MessagingEngine,
    event_tx: &mpsc::UnboundedSender<ConnectionEvent>,
    remote_device: &Device,
    pending_send: PendingSendText,
) -> Result<(), String> {
    let PendingSendText {
        conversation_id,
        message_id,
        sent_at_unix_ms,
        body,
        response,
    } = pending_send;

    engine
        .send_message(
            session,
            message_id.clone(),
            conversation_id,
            MessageKind::Text,
            sent_at_unix_ms,
            body.into_bytes(),
        )
        .await
        .map_err(|error| error.to_string())?;

    let mut outbound_acknowledged = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, engine.receive_next(session)).await {
            Ok(Ok(outcome)) => {
                emit_delivered_messages(event_tx, remote_device, &outcome);
                if outcome
                    .acknowledged_message_ids()
                    .iter()
                    .any(|ack_id| ack_id == &message_id)
                {
                    outbound_acknowledged = true;
                    break;
                }
            }
            Ok(Err(error)) => {
                let _ = response.send(Err(error.to_string()));
                return Err(error.to_string());
            }
            Err(_) => break,
        }
    }

    let _ = response.send(Ok(RealSendOutcome {
        outbound_acknowledged,
        forward_secrecy_active: session
            .forward_secrecy_state()
            .sending_chain_next_message_number()
            .is_some(),
    }));
    Ok(())
}

fn emit_delivered_messages(
    event_tx: &mpsc::UnboundedSender<ConnectionEvent>,
    remote_device: &Device,
    outcome: &localmessenger_messaging::ReceiveOutcome,
) {
    for delivered in outcome.delivered_messages() {
        let body = String::from_utf8_lossy(delivered.body()).into_owned();
        let _ = event_tx.send(ConnectionEvent::InboundMessage {
            device_id: remote_device.device_id().to_string(),
            conversation_id: delivered.conversation_id().to_string(),
            message_id: delivered.message_id().to_string(),
            body,
            sent_at_unix_ms: delivered.sent_at_unix_ms(),
        });
    }
}

fn remote_device_from_invite(invite: &DeviceContactInvite) -> Result<Device, String> {
    let device_id = DeviceId::new(invite.device_id.clone()).map_err(|error| error.to_string())?;
    let member_id = MemberId::new(invite.member_id.clone()).map_err(|error| error.to_string())?;
    Device::new(
        device_id,
        member_id,
        invite.display_name.clone(),
        invite.identity_keys.clone(),
    )
    .map_err(|error| error.to_string())
}
