use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use localmessenger_core::{Device, DeviceId, MemberId, MemberProfile};
use localmessenger_crypto::{IdentityKeyMaterial, IdentityKeyPair, LocalPrekeyStore};
use localmessenger_discovery::{DiscoveredPeer, PeerCapability};
use localmessenger_messaging::{
    GroupEncryptedMessage, GroupMembership, GroupParticipant, GroupSession, InMemoryFrameChannel,
    MessageKind, MessagingEngine, SecureSession, SessionInitiator, SessionResponder,
};
use localmessenger_transport::TransportIdentity;
use rand_core::OsRng;
use std::collections::BTreeMap;

pub struct BootstrapRuntime {
    pub member: MemberProfile,
    pub runtime: DirectChatRuntime,
}

pub struct DirectChatRuntime {
    remote_display_name: String,
    remote_device_id: DeviceId,
    peer: DiscoveredPeer,
    local_session: SecureSession,
    local_engine: MessagingEngine,
    remote_session: SecureSession,
    remote_engine: MessagingEngine,
    reply_script: Vec<String>,
    next_reply_index: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeInboundMessage {
    pub message_id: String,
    pub body: String,
    pub sent_at_unix_ms: i64,
}

#[derive(Debug, Clone)]
pub struct RuntimeSendOutcome {
    pub outbound_acknowledged: bool,
    pub inbound_messages: Vec<RuntimeInboundMessage>,
    pub forward_secrecy_active: bool,
}

impl DirectChatRuntime {
    pub fn remote_display_name(&self) -> &str {
        &self.remote_display_name
    }

    pub fn peer(&self) -> &DiscoveredPeer {
        &self.peer
    }

    pub fn remote_device_id(&self) -> &DeviceId {
        &self.remote_device_id
    }

    pub async fn send_text(
        &mut self,
        conversation_id: &str,
        outbound_message_id: String,
        sent_at_unix_ms: i64,
        body: String,
    ) -> Result<RuntimeSendOutcome, String> {
        self.local_engine
            .send_message(
                &mut self.local_session,
                outbound_message_id,
                conversation_id.to_string(),
                MessageKind::Text,
                sent_at_unix_ms,
                body.into_bytes(),
            )
            .await
            .map_err(|error| error.to_string())?;

        self.remote_engine
            .receive_next(&mut self.remote_session)
            .await
            .map_err(|error| error.to_string())?;

        let ack_outcome = self
            .local_engine
            .receive_next(&mut self.local_session)
            .await
            .map_err(|error| error.to_string())?;

        let mut inbound_messages = Vec::new();
        if let Some(reply_body) = self.next_reply_body() {
            let reply_message_id = format!(
                "{}-{}-reply-{}",
                conversation_id, self.remote_device_id, self.next_reply_index
            );
            let reply_sent_at = sent_at_unix_ms.saturating_add(1);

            self.remote_engine
                .send_message(
                    &mut self.remote_session,
                    reply_message_id.clone(),
                    conversation_id.to_string(),
                    MessageKind::Text,
                    reply_sent_at,
                    reply_body.clone().into_bytes(),
                )
                .await
                .map_err(|error| error.to_string())?;

            let inbound = self
                .local_engine
                .receive_next(&mut self.local_session)
                .await
                .map_err(|error| error.to_string())?;
            self.remote_engine
                .receive_next(&mut self.remote_session)
                .await
                .map_err(|error| error.to_string())?;

            for delivered in inbound.delivered_messages() {
                inbound_messages.push(RuntimeInboundMessage {
                    message_id: delivered.message_id().to_string(),
                    body: String::from_utf8_lossy(delivered.body()).into_owned(),
                    sent_at_unix_ms: delivered.sent_at_unix_ms(),
                });
            }
        }

        Ok(RuntimeSendOutcome {
            outbound_acknowledged: !ack_outcome.acknowledged_message_ids().is_empty()
                && self.local_engine.pending_count() == 0,
            inbound_messages,
            forward_secrecy_active: self
                .local_session
                .forward_secrecy_state()
                .sending_chain_next_message_number()
                .is_some(),
        })
    }

    pub fn engine_snapshot(&self) -> localmessenger_messaging::PendingQueueSnapshot {
        self.local_engine.export_pending_queue()
    }

    fn next_reply_body(&mut self) -> Option<String> {
        if self.reply_script.is_empty() {
            return None;
        }

        let index = (self.next_reply_index as usize) % self.reply_script.len();
        let body = self.reply_script[index].clone();
        self.next_reply_index = self.next_reply_index.saturating_add(1);
        Some(body)
    }
}

// ─────────────────────────── Group chat runtime ──────────────────────────────

/// Specification for one remote member when bootstrapping a group session.
pub struct GroupRemoteMemberSpec<'a> {
    pub member_id: &'a str,
    pub display_name: &'a str,
    pub device_id: &'a str,
    pub device_name: &'a str,
    /// Seed passed to `LocalPrekeyStore::generate` so the demo is deterministic.
    pub prekey_seed: u32,
    /// Lines the remote member will reply with (round-robin, empty = no reply).
    pub reply_script: Vec<&'a str>,
}

struct GroupMemberPair {
    remote_display_name: String,
    local_session: SecureSession,
    local_engine: MessagingEngine,
    remote_session: SecureSession,
    remote_engine: MessagingEngine,
    remote_group_session: GroupSession,
    reply_script: Vec<String>,
    next_reply_index: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeInboundGroupMessage {
    pub message_id: String,
    pub author: String,
    pub body: String,
    pub sent_at_unix_ms: i64,
}

#[derive(Debug, Clone)]
pub struct GroupSendOutcome {
    pub members_reached: usize,
    pub inbound_messages: Vec<RuntimeInboundGroupMessage>,
}

pub struct GroupChatRuntime {
    local_group_session: GroupSession,
    member_pairs: BTreeMap<String, GroupMemberPair>,
    pub local_display_name: String,
}

impl GroupChatRuntime {
    pub fn member_count(&self) -> usize {
        self.member_pairs.len()
    }

    pub fn epoch(&self) -> u64 {
        self.local_group_session.epoch()
    }

    /// Encrypt the plaintext `body`, fan it out to every member over their
    /// individual pairwise sessions, and collect any simulated replies.
    pub async fn send_text(
        &mut self,
        conversation_id: &str,
        message_id: String,
        sent_at_unix_ms: i64,
        body: String,
    ) -> Result<GroupSendOutcome, String> {
        // Encrypt with our local sender key.
        let encrypted = self
            .local_group_session
            .encrypt_message(
                message_id.clone(),
                MessageKind::Text,
                sent_at_unix_ms,
                body.into_bytes(),
            )
            .map_err(|e| e.to_string())?;
        let encrypted_bytes = encrypted.encode().map_err(|e| e.to_string())?;

        let mut members_reached = 0_usize;
        let mut inbound_messages: Vec<RuntimeInboundGroupMessage> = Vec::new();

        let member_keys: Vec<String> = self.member_pairs.keys().cloned().collect();

        for member_key in &member_keys {
            let pair = self.member_pairs.get_mut(member_key).unwrap();

            // ── Send the group-encrypted payload via the pairwise channel ──
            let fanout_id = format!("{message_id}-to-{member_key}");
            pair.local_engine
                .send_message(
                    &mut pair.local_session,
                    fanout_id,
                    conversation_id.to_string(),
                    MessageKind::System, // body = opaque group-encrypted blob
                    sent_at_unix_ms,
                    encrypted_bytes.clone(),
                )
                .await
                .map_err(|e| e.to_string())?;

            // Remote receives.
            let remote_outcome = pair
                .remote_engine
                .receive_next(&mut pair.remote_session)
                .await
                .map_err(|e| e.to_string())?;

            // Local receives ACK.
            let _ = pair
                .local_engine
                .receive_next(&mut pair.local_session)
                .await
                .map_err(|e| e.to_string())?;

            // Remote validates by decrypting via its own GroupSession.
            for delivered in remote_outcome.delivered_messages() {
                if let Ok(msg) = GroupEncryptedMessage::decode(delivered.body()) {
                    let _ = pair.remote_group_session.decrypt_message(&msg);
                }
            }
            members_reached += 1;

            // ── Simulated reply ────────────────────────────────────────────
            let reply_body = if pair.reply_script.is_empty() {
                None
            } else {
                let idx = (pair.next_reply_index as usize) % pair.reply_script.len();
                let body = pair.reply_script[idx].clone();
                pair.next_reply_index += 1;
                Some(body)
            };

            if let Some(reply_text) = reply_body {
                let reply_id = format!("grp-reply-{}-{}", member_key, pair.next_reply_index);
                let reply_sent_at = sent_at_unix_ms.saturating_add(1);

                // Remote encrypts reply via its GroupSession.
                let reply_encrypted = pair
                    .remote_group_session
                    .encrypt_message(
                        reply_id.clone(),
                        MessageKind::Text,
                        reply_sent_at,
                        reply_text.clone().into_bytes(),
                    )
                    .map_err(|e| e.to_string())?;
                let reply_bytes = reply_encrypted.encode().map_err(|e| e.to_string())?;

                // Remote sends reply to local via pairwise channel.
                let reply_fanout_id = format!("{reply_id}-to-local");
                pair.remote_engine
                    .send_message(
                        &mut pair.remote_session,
                        reply_fanout_id,
                        conversation_id.to_string(),
                        MessageKind::System,
                        reply_sent_at,
                        reply_bytes,
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                // Local receives reply.
                let local_outcome = pair
                    .local_engine
                    .receive_next(&mut pair.local_session)
                    .await
                    .map_err(|e| e.to_string())?;

                // Remote receives its own ACK.
                let _ = pair
                    .remote_engine
                    .receive_next(&mut pair.remote_session)
                    .await
                    .map_err(|e| e.to_string())?;

                // Local decrypts via its GroupSession.
                // We need to temporarily take the session out and put it back.
                let remote_display_name = pair.remote_display_name.clone();
                for delivered in local_outcome.delivered_messages() {
                    if let Ok(group_msg) = GroupEncryptedMessage::decode(delivered.body()) {
                        if let Ok(decrypted) = self.local_group_session.decrypt_message(&group_msg)
                        {
                            inbound_messages.push(RuntimeInboundGroupMessage {
                                message_id: decrypted.message_id().to_string(),
                                author: remote_display_name.clone(),
                                body: String::from_utf8_lossy(decrypted.body()).into_owned(),
                                sent_at_unix_ms: decrypted.sent_at_unix_ms(),
                            });
                        }
                    }
                }
            }
        }

        Ok(GroupSendOutcome {
            members_reached,
            inbound_messages,
        })
    }
}

/// Bootstrap a complete group chat runtime with in-memory sender-key exchange
/// and pairwise QUIC loopback sessions for every remote member.
pub async fn bootstrap_group_chat_runtime(
    local_device: &Device,
    local_identity_material: &IdentityKeyMaterial,
    group_id: &str,
    local_display_name: &str,
    remote_specs: Vec<GroupRemoteMemberSpec<'_>>,
) -> Result<GroupChatRuntime, String> {
    let mut rng = OsRng;

    // ── Build a shared GroupMembership ─────────────────────────────────────
    let local_participant = GroupParticipant::from_device(local_device);
    let mut all_participants = vec![local_participant];

    struct RemoteInfo {
        device: Device,
        identity: IdentityKeyPair,
        display_name: String,
        device_id_str: String,
        prekey_seed: u32,
        reply_script: Vec<String>,
    }

    let mut remote_infos: Vec<RemoteInfo> = Vec::new();
    for spec in &remote_specs {
        let member_id_value = MemberId::new(spec.member_id).map_err(|e| e.to_string())?;
        let device_id_value = DeviceId::new(spec.device_id).map_err(|e| e.to_string())?;
        let identity = IdentityKeyPair::generate(&mut rng);
        let device = Device::from_identity_keypair(
            device_id_value,
            member_id_value,
            spec.device_name,
            &identity,
        )
        .map_err(|e| e.to_string())?;
        all_participants.push(GroupParticipant::from_device(&device));
        remote_infos.push(RemoteInfo {
            device,
            identity,
            display_name: spec.display_name.to_string(),
            device_id_str: spec.device_id.to_string(),
            prekey_seed: spec.prekey_seed,
            reply_script: spec.reply_script.iter().map(|s| s.to_string()).collect(),
        });
    }

    let membership = GroupMembership::new(all_participants).map_err(|e| e.to_string())?;

    // ── Create local GroupSession ──────────────────────────────────────────
    let mut local_group_session = GroupSession::create(
        &mut rng,
        group_id,
        0, // epoch 0
        local_device.clone(),
        membership.clone(),
    )
    .map_err(|e| e.to_string())?;

    let local_distribution = local_group_session.sender_key_distribution();

    // ── Set up one GroupMemberPair per remote member ───────────────────────
    let mut member_pairs: BTreeMap<String, GroupMemberPair> = BTreeMap::new();

    for info in remote_infos {
        // Create remote GroupSession.
        let mut remote_group_session = GroupSession::create(
            &mut rng,
            group_id,
            0,
            info.device.clone(),
            membership.clone(),
        )
        .map_err(|e| e.to_string())?;

        // In-memory sender-key exchange (no network round-trip needed for demo).
        remote_group_session
            .import_sender_key(local_distribution.clone())
            .map_err(|e| e.to_string())?;
        let remote_distribution = remote_group_session.sender_key_distribution();
        local_group_session
            .import_sender_key(remote_distribution)
            .map_err(|e| e.to_string())?;

        // Establish a pairwise QUIC loopback session for message fan-out.
        let server_identity =
            TransportIdentity::generate("group-runtime.local").map_err(|e| e.to_string())?;
        let (local_channel, remote_channel) = InMemoryFrameChannel::pair();

        let remote_prekeys = LocalPrekeyStore::generate(
            &mut rng,
            &info.identity,
            info.prekey_seed,
            4,
            info.prekey_seed * 100,
        );
        let mut responder = SessionResponder::new(
            info.device.clone(),
            info.identity,
            remote_prekeys,
            &server_identity.certificate_der,
        )
        .map_err(|e| e.to_string())?;
        let offer = responder
            .remote_session_offer()
            .map_err(|e| e.to_string())?;

        let accept_task = tokio::spawn(async move {
            responder
                .accept(remote_channel)
                .await
                .map_err(|e| e.to_string())
        });

        let initiator = SessionInitiator::new(
            local_device.clone(),
            IdentityKeyPair::from_material(local_identity_material),
        )
        .map_err(|e| e.to_string())?;
        let local_session = initiator
            .establish(local_channel, &offer, &server_identity.certificate_der)
            .await
            .map_err(|e| e.to_string())?;
        let remote_session = accept_task.await.map_err(|e| e.to_string())??;

        let local_engine = MessagingEngine::from_session(&local_session);
        let remote_engine = MessagingEngine::from_session(&remote_session);

        member_pairs.insert(
            info.device_id_str,
            GroupMemberPair {
                remote_display_name: info.display_name,
                local_session,
                local_engine,
                remote_session,
                remote_engine,
                remote_group_session,
                reply_script: info.reply_script,
                next_reply_index: 0,
            },
        );
    }

    Ok(GroupChatRuntime {
        local_group_session,
        member_pairs,
        local_display_name: local_display_name.to_string(),
    })
}

// ─────────────────────────── Direct chat runtime ─────────────────────────────

pub async fn bootstrap_direct_chat_runtime(
    local_device: &Device,
    local_identity_material: &IdentityKeyMaterial,
    remote_member_id: &str,
    remote_display_name: &str,
    remote_device_id: &str,
    remote_device_name: &str,
    prekey_seed: u32,
    capabilities: Vec<PeerCapability>,
    reply_script: Vec<&str>,
) -> Result<BootstrapRuntime, String> {
    let mut rng = OsRng;
    let remote_member_id_value =
        MemberId::new(remote_member_id).map_err(|error| error.to_string())?;
    let remote_device_id_value =
        DeviceId::new(remote_device_id).map_err(|error| error.to_string())?;
    let remote_identity = IdentityKeyPair::generate(&mut rng);
    let remote_device = Device::from_identity_keypair(
        remote_device_id_value.clone(),
        remote_member_id_value.clone(),
        remote_device_name,
        &remote_identity,
    )
    .map_err(|error| error.to_string())?;

    let mut remote_member = MemberProfile::new(remote_member_id_value, remote_display_name)
        .map_err(|error| error.to_string())?;
    remote_member
        .add_device(remote_device.clone())
        .map_err(|error| error.to_string())?;

    let server_identity =
        TransportIdentity::generate("runtime.local").map_err(|error| error.to_string())?;
    let (client_channel, server_channel) = InMemoryFrameChannel::pair();

    let remote_prekeys = LocalPrekeyStore::generate(
        &mut rng,
        &remote_identity,
        prekey_seed,
        4,
        prekey_seed * 100,
    );
    let mut responder = SessionResponder::new(
        remote_device.clone(),
        remote_identity,
        remote_prekeys,
        &server_identity.certificate_der,
    )
    .map_err(|error| error.to_string())?;
    let offer = responder
        .remote_session_offer()
        .map_err(|error| error.to_string())?;

    let accept_task = tokio::spawn(async move {
        responder
            .accept(server_channel)
            .await
            .map_err(|error| error.to_string())
    });

    let initiator = SessionInitiator::new(
        local_device.clone(),
        IdentityKeyPair::from_material(local_identity_material),
    )
    .map_err(|error| error.to_string())?;
    let local_session = initiator
        .establish(client_channel, &offer, &server_identity.certificate_der)
        .await
        .map_err(|error| error.to_string())?;
    let remote_session = accept_task.await.map_err(|error| error.to_string())??;

    let peer = DiscoveredPeer {
        service_instance: format!("{remote_device_name}-{remote_device_id}.runtime.local"),
        member_id: remote_member.member_id().clone(),
        device_id: remote_device_id_value.clone(),
        device_name: remote_device_name.to_string(),
        port: 7443,
        socket_address: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7443)),
        hostname: Some("runtime.local".to_string()),
        capabilities,
    };

    Ok(BootstrapRuntime {
        member: remote_member,
        runtime: DirectChatRuntime {
            remote_display_name: remote_display_name.to_string(),
            remote_device_id: remote_device_id_value,
            peer,
            local_engine: MessagingEngine::from_session(&local_session),
            local_session,
            remote_engine: MessagingEngine::from_session(&remote_session),
            remote_session,
            reply_script: reply_script.into_iter().map(str::to_string).collect(),
            next_reply_index: 0,
        },
    })
}
