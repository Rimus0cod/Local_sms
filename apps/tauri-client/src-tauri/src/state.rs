use std::collections::BTreeMap;

use localmessenger_core::{
    Device, DeviceId, MemberId, MemberProfile, VerificationMethod, VerificationStatus,
};
use localmessenger_crypto::IdentityKeyPair;
use localmessenger_discovery::PeerCapability;
use rand_core::OsRng;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::runtime::{DirectChatRuntime, bootstrap_direct_chat_runtime};

pub type SharedClientState = Mutex<ClientState>;

pub struct ClientState {
    local_profile: MemberProfile,
    local_device_id: DeviceId,
    contacts: Vec<MemberProfile>,
    contact_runtimes: BTreeMap<String, DirectChatRuntime>,
    chat_runtime_device_ids: BTreeMap<String, String>,
    chats: Vec<ChatThreadView>,
    message_counter: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum VerificationAction {
    Qr,
    Safety,
}

impl VerificationAction {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "qr" => Ok(Self::Qr),
            "safety" => Ok(Self::Safety),
            _ => Err(format!("unsupported verification action '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSnapshot {
    pub transport_status: TransportStatusView,
    pub local_profile: LocalProfileView,
    pub chats: Vec<ChatThreadView>,
    pub peers: Vec<PeerView>,
    pub verification: VerificationWorkspaceView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportStatusView {
    pub discovery_mode: String,
    pub transport_mode: String,
    pub crypto_mode: String,
    pub storage_mode: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProfileView {
    pub display_name: String,
    pub active_device_name: String,
    pub active_device_id: String,
    pub trusted_device_count: usize,
    pub total_device_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatThreadView {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub presence_label: String,
    pub unread_count: u32,
    pub security_label: String,
    pub kind: ChatKindView,
    pub participants: Vec<String>,
    pub messages: Vec<MessageView>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatKindView {
    Direct,
    Group,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageView {
    pub id: String,
    pub author: String,
    pub body: String,
    pub timestamp_label: String,
    pub direction: MessageDirectionView,
    pub delivery_state: DeliveryStateView,
    pub reply_preview: Option<String>,
    pub reactions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirectionView {
    Inbound,
    Outbound,
    System,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStateView {
    Queued,
    Sent,
    Delivered,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerView {
    pub member_id: String,
    pub device_id: String,
    pub device_name: String,
    pub endpoint: String,
    pub hostname: Option<String>,
    pub capabilities: Vec<String>,
    pub state: PeerStateCode,
    pub trust_label: String,
    pub last_seen_label: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerStateCode {
    Live,
    Reconnecting,
    Dormant,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationWorkspaceView {
    pub trusted_device_count: usize,
    pub pending_device_count: usize,
    pub devices: Vec<VerificationDeviceView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationDeviceView {
    pub member_id: String,
    pub member_name: String,
    pub device_id: String,
    pub device_name: String,
    pub state: VerificationStateCode,
    pub method: Option<VerificationMethodCode>,
    pub safety_number: String,
    pub qr_payload_hex: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStateCode {
    Pending,
    Verified,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethodCode {
    QrCode,
    SafetyNumber,
}

impl ClientState {
    pub async fn bootstrap() -> Result<Self, String> {
        let mut rng = OsRng;

        let rimus_id = MemberId::new("rimus").map_err(|error| error.to_string())?;
        let mut rimus =
            MemberProfile::new(rimus_id.clone(), "Rimus").map_err(|error| error.to_string())?;

        let local_identity = IdentityKeyPair::generate(&mut rng);
        let local_identity_material = local_identity.to_material();
        let mut rimus_laptop = Device::from_identity_keypair(
            DeviceId::new("rimus-laptop").map_err(|error| error.to_string())?,
            rimus_id.clone(),
            "Rimus Laptop",
            &local_identity,
        )
        .map_err(|error| error.to_string())?;
        let laptop_qr = rimus_laptop
            .qr_payload(None)
            .map_err(|error| error.to_string())?;
        rimus_laptop
            .verify_with_qr_payload(&laptop_qr)
            .map_err(|error| error.to_string())?;
        let local_device_id = rimus_laptop.device_id().clone();
        let local_reference = rimus_laptop.clone();

        let rimus_phone_identity = IdentityKeyPair::generate(&mut rng);
        let mut rimus_phone = Device::from_identity_keypair(
            DeviceId::new("rimus-phone").map_err(|error| error.to_string())?,
            rimus_id,
            "Rimus Phone",
            &rimus_phone_identity,
        )
        .map_err(|error| error.to_string())?;
        let phone_qr = rimus_phone
            .qr_payload(Some(&local_reference))
            .map_err(|error| error.to_string())?;
        rimus_phone
            .verify_with_qr_payload(&phone_qr)
            .map_err(|error| error.to_string())?;

        rimus
            .add_device(rimus_laptop)
            .map_err(|error| error.to_string())?;
        rimus
            .add_device(rimus_phone)
            .map_err(|error| error.to_string())?;

        let bob_bootstrap = bootstrap_direct_chat_runtime(
            &local_reference,
            &local_identity_material,
            "bob",
            "Bob",
            "bob-phone",
            "Bob Phone",
            41,
            vec![PeerCapability::MessagingV1, PeerCapability::PresenceV1],
            vec![
                "QUIC lane is clear on my side.",
                "ACK path still looks clean after the latest ratchet step.",
            ],
        )
        .await?;
        let mut bob = bob_bootstrap.member;
        let bob_phone_id = DeviceId::new("bob-phone").map_err(|error| error.to_string())?;
        let bob_phone_safety = bob
            .device(&bob_phone_id)
            .ok_or_else(|| "missing Bob phone".to_string())?
            .safety_number_with(&local_reference);
        bob.verify_device_by_safety_number(&bob_phone_id, &local_reference, &bob_phone_safety)
            .map_err(|error| error.to_string())?;

        let carol_bootstrap = bootstrap_direct_chat_runtime(
            &local_reference,
            &local_identity_material,
            "carol",
            "Carol",
            "carol-workstation",
            "Carol Workstation",
            51,
            vec![
                PeerCapability::MessagingV1,
                PeerCapability::FileTransferV1,
                PeerCapability::PresenceV1,
            ],
            vec![
                "I can see the session update now.",
                "Let's keep attachments disabled until every device is verified.",
            ],
        )
        .await?;
        let mut carol = carol_bootstrap.member;
        let carol_device_id =
            DeviceId::new("carol-workstation").map_err(|error| error.to_string())?;
        let carol_qr = carol
            .device(&carol_device_id)
            .ok_or_else(|| "missing Carol workstation".to_string())?
            .qr_payload(Some(&local_reference))
            .map_err(|error| error.to_string())?;
        carol
            .verify_device_by_qr(&carol_device_id, &carol_qr)
            .map_err(|error| error.to_string())?;

        let daria_bootstrap = bootstrap_direct_chat_runtime(
            &local_reference,
            &local_identity_material,
            "daria",
            "Daria",
            "daria-laptop",
            "Daria Laptop",
            61,
            vec![PeerCapability::MessagingV1, PeerCapability::PresenceV1],
            vec!["Ready for verification when you are."],
        )
        .await?;
        let daria = daria_bootstrap.member;

        let contacts = vec![bob, carol, daria];

        let mut contact_runtimes = BTreeMap::new();
        contact_runtimes.insert(
            bob_bootstrap.runtime.remote_device_id().to_string(),
            bob_bootstrap.runtime,
        );
        contact_runtimes.insert(
            carol_bootstrap.runtime.remote_device_id().to_string(),
            carol_bootstrap.runtime,
        );
        contact_runtimes.insert(
            daria_bootstrap.runtime.remote_device_id().to_string(),
            daria_bootstrap.runtime,
        );

        let chats = vec![
            ChatThreadView {
                id: "chat-bob".to_string(),
                title: "Bob".to_string(),
                summary: "Secure runtime session is active on QUIC loopback.".to_string(),
                presence_label: "secure session active".to_string(),
                unread_count: 0,
                security_label: "Verified device pair".to_string(),
                kind: ChatKindView::Direct,
                participants: vec!["Rimus".to_string(), "Bob".to_string()],
                messages: vec![
                    MessageView {
                        id: "m-1".to_string(),
                        author: "Bob".to_string(),
                        body: "I am back on the local runtime. QUIC path is stable now."
                            .to_string(),
                        timestamp_label: "09:18".to_string(),
                        direction: MessageDirectionView::Inbound,
                        delivery_state: DeliveryStateView::Delivered,
                        reply_preview: None,
                        reactions: vec!["ack".to_string()],
                    },
                    MessageView {
                        id: "m-2".to_string(),
                        author: "Rimus".to_string(),
                        body: "Good. I am wiring the desktop client to the secure backend."
                            .to_string(),
                        timestamp_label: "09:22".to_string(),
                        direction: MessageDirectionView::Outbound,
                        delivery_state: DeliveryStateView::Delivered,
                        reply_preview: Some(
                            "I am back on the local runtime. QUIC path is stable now."
                                .to_string(),
                        ),
                        reactions: Vec::new(),
                    },
                ],
            },
            ChatThreadView {
                id: "chat-lan-crew".to_string(),
                title: "LAN Crew".to_string(),
                summary: "Sender-key orchestration is staged; direct sessions are live."
                    .to_string(),
                presence_label: "desktop group bridge staged".to_string(),
                unread_count: 0,
                security_label: "Group sender key epoch 4".to_string(),
                kind: ChatKindView::Group,
                participants: vec![
                    "Rimus".to_string(),
                    "Bob".to_string(),
                    "Carol".to_string(),
                    "Daria".to_string(),
                ],
                messages: vec![
                    MessageView {
                        id: "g-1".to_string(),
                        author: "System".to_string(),
                        body: "Desktop client is using live pairwise sessions; group fan-out is staged."
                            .to_string(),
                        timestamp_label: "08:41".to_string(),
                        direction: MessageDirectionView::System,
                        delivery_state: DeliveryStateView::Delivered,
                        reply_preview: None,
                        reactions: Vec::new(),
                    },
                    MessageView {
                        id: "g-2".to_string(),
                        author: "Carol".to_string(),
                        body: "Keep the group pane read-only until sender-key fan-out is wired."
                            .to_string(),
                        timestamp_label: "08:45".to_string(),
                        direction: MessageDirectionView::Inbound,
                        delivery_state: DeliveryStateView::Delivered,
                        reply_preview: None,
                        reactions: Vec::new(),
                    },
                ],
            },
            ChatThreadView {
                id: "chat-carol".to_string(),
                title: "Carol".to_string(),
                summary: "Attachments stay locked until the verification workspace is green."
                    .to_string(),
                presence_label: "secure session active".to_string(),
                unread_count: 0,
                security_label: "Verified device pair".to_string(),
                kind: ChatKindView::Direct,
                participants: vec!["Rimus".to_string(), "Carol".to_string()],
                messages: vec![MessageView {
                    id: "c-1".to_string(),
                    author: "Carol".to_string(),
                    body: "Attachments stay locked until the verification workspace is green."
                        .to_string(),
                    timestamp_label: "Yesterday".to_string(),
                    direction: MessageDirectionView::Inbound,
                    delivery_state: DeliveryStateView::Delivered,
                    reply_preview: None,
                    reactions: Vec::new(),
                }],
            },
        ];

        let chat_runtime_device_ids = BTreeMap::from([
            ("chat-bob".to_string(), "bob-phone".to_string()),
            ("chat-carol".to_string(), "carol-workstation".to_string()),
        ]);

        Ok(Self {
            local_profile: rimus,
            local_device_id,
            contacts,
            contact_runtimes,
            chat_runtime_device_ids,
            chats,
            message_counter: 100,
        })
    }

    pub fn snapshot(&self) -> ClientSnapshot {
        let active_device = self
            .local_profile
            .device(&self.local_device_id)
            .map(|device| device.device_name().to_string())
            .unwrap_or_else(|| "Unknown Device".to_string());

        ClientSnapshot {
            transport_status: TransportStatusView {
                discovery_mode: "Runtime peer registry (mDNS adapter pending)".to_string(),
                transport_mode: "QUIC transport".to_string(),
                crypto_mode: "X3DH bootstrap + Double Ratchet".to_string(),
                storage_mode: "Encrypted storage backend pending client binding".to_string(),
            },
            local_profile: LocalProfileView {
                display_name: self.local_profile.display_name().to_string(),
                active_device_name: active_device,
                active_device_id: self.local_device_id.to_string(),
                trusted_device_count: self.local_profile.verified_devices().len(),
                total_device_count: self.local_profile.devices().count(),
            },
            chats: self.chats.clone(),
            peers: self.peer_views(),
            verification: self.verification_workspace(),
        }
    }

    pub fn refresh_peer_discovery(&mut self) {
        self.sync_chat_labels();
    }

    pub async fn send_message(&mut self, chat_id: &str, body: &str) -> Result<(), String> {
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Err("message body cannot be empty".to_string());
        }

        let chat = self
            .chats
            .iter()
            .find(|chat| chat.id == chat_id)
            .ok_or_else(|| format!("chat '{chat_id}' not found"))?;
        if matches!(chat.kind, ChatKindView::Group) {
            return Err("group messaging is still staged in the desktop client".to_string());
        }

        let remote_device_id = self
            .chat_runtime_device_ids
            .get(chat_id)
            .cloned()
            .ok_or_else(|| format!("chat '{chat_id}' is not bound to a runtime session"))?;
        self.message_counter = self.message_counter.saturating_add(1);
        let outbound_message_id = format!("local-{}", self.message_counter);
        let sent_at_unix_ms = now_unix_ms();

        let (remote_author, outcome) = {
            let runtime = self
                .contact_runtimes
                .get_mut(&remote_device_id)
                .ok_or_else(|| format!("runtime for device '{remote_device_id}' is missing"))?;
            let remote_author = runtime.remote_display_name().to_string();
            let outcome = runtime
                .send_text(
                    chat_id,
                    outbound_message_id.clone(),
                    sent_at_unix_ms,
                    trimmed.to_string(),
                )
                .await?;
            (remote_author, outcome)
        };

        let security_label =
            self.security_label_for_device(&remote_device_id, outcome.forward_secrecy_active);

        let chat = self
            .chats
            .iter_mut()
            .find(|chat| chat.id == chat_id)
            .ok_or_else(|| format!("chat '{chat_id}' not found"))?;
        chat.messages.push(MessageView {
            id: outbound_message_id,
            author: self.local_profile.display_name().to_string(),
            body: trimmed.to_string(),
            timestamp_label: timestamp_label(sent_at_unix_ms),
            direction: MessageDirectionView::Outbound,
            delivery_state: if outcome.outbound_acknowledged {
                DeliveryStateView::Delivered
            } else {
                DeliveryStateView::Sent
            },
            reply_preview: None,
            reactions: Vec::new(),
        });

        for inbound in outcome.inbound_messages {
            chat.messages.push(MessageView {
                id: inbound.message_id,
                author: remote_author.clone(),
                body: inbound.body,
                timestamp_label: timestamp_label(inbound.sent_at_unix_ms),
                direction: MessageDirectionView::Inbound,
                delivery_state: DeliveryStateView::Delivered,
                reply_preview: Some(trimmed.to_string()),
                reactions: Vec::new(),
            });
        }

        if let Some(last_message) = chat.messages.last() {
            chat.summary = preview(&last_message.body);
        }
        chat.unread_count = 0;
        chat.presence_label = "secure session active".to_string();
        chat.security_label = security_label;

        Ok(())
    }

    pub fn verify_device(
        &mut self,
        device_id: &str,
        action: VerificationAction,
    ) -> Result<(), String> {
        let local_reference = self
            .local_profile
            .device(&self.local_device_id)
            .cloned()
            .ok_or_else(|| "local active device is missing".to_string())?;
        let (member_index, target_device_id) = self
            .find_remote_device(device_id)
            .ok_or_else(|| format!("device '{device_id}' not found"))?;

        match action {
            VerificationAction::Qr => {
                let payload = self.contacts[member_index]
                    .device(&target_device_id)
                    .ok_or_else(|| "target device disappeared".to_string())?
                    .qr_payload(Some(&local_reference))
                    .map_err(|error| error.to_string())?;
                self.contacts[member_index]
                    .verify_device_by_qr(&target_device_id, &payload)
                    .map_err(|error| error.to_string())?;
            }
            VerificationAction::Safety => {
                let safety_number = self.contacts[member_index]
                    .device(&target_device_id)
                    .ok_or_else(|| "target device disappeared".to_string())?
                    .safety_number_with(&local_reference);
                self.contacts[member_index]
                    .verify_device_by_safety_number(
                        &target_device_id,
                        &local_reference,
                        &safety_number,
                    )
                    .map_err(|error| error.to_string())?;
            }
        }

        self.sync_chat_labels();
        Ok(())
    }

    fn peer_views(&self) -> Vec<PeerView> {
        let mut peers = Vec::new();

        for member in &self.contacts {
            for device in member.devices() {
                let runtime = self.contact_runtimes.get(device.device_id().as_str());
                let (endpoint, hostname, capabilities, state, last_seen_label) =
                    if let Some(runtime) = runtime {
                        (
                            runtime
                                .peer()
                                .endpoint()
                                .map(|addr| addr.to_string())
                                .unwrap_or_else(|| "unknown endpoint".to_string()),
                            runtime.peer().hostname.clone(),
                            runtime
                                .peer()
                                .capabilities
                                .iter()
                                .map(|capability| capability.as_str().to_string())
                                .collect(),
                            if device.is_verified() {
                                PeerStateCode::Live
                            } else {
                                PeerStateCode::Reconnecting
                            },
                            if device.is_verified() {
                                "runtime session active".to_string()
                            } else {
                                "runtime reachable, verification pending".to_string()
                            },
                        )
                    } else {
                        (
                            "unknown endpoint".to_string(),
                            None,
                            Vec::new(),
                            PeerStateCode::Dormant,
                            "no active runtime".to_string(),
                        )
                    };

                peers.push(PeerView {
                    member_id: member.member_id().to_string(),
                    device_id: device.device_id().to_string(),
                    device_name: device.device_name().to_string(),
                    endpoint,
                    hostname,
                    capabilities,
                    state,
                    trust_label: if device.is_verified() {
                        "verified".to_string()
                    } else {
                        "pending".to_string()
                    },
                    last_seen_label,
                });
            }
        }

        peers
    }

    fn verification_workspace(&self) -> VerificationWorkspaceView {
        let local_reference = self.local_profile.device(&self.local_device_id);
        let devices: Vec<VerificationDeviceView> = self
            .contacts
            .iter()
            .flat_map(|member| {
                member.devices().map(move |device| {
                    let safety_number = local_reference
                        .map(|local| device.safety_number_with(local).digits())
                        .unwrap_or_else(|| "unavailable".to_string());
                    let qr_payload_hex = device
                        .qr_payload(local_reference)
                        .map(|bytes| hex_encode(&bytes))
                        .unwrap_or_else(|_| "qr-unavailable".to_string());

                    VerificationDeviceView {
                        member_id: member.member_id().to_string(),
                        member_name: member.display_name().to_string(),
                        device_id: device.device_id().to_string(),
                        device_name: device.device_name().to_string(),
                        state: verification_state_code(device.verification_status()),
                        method: verification_method_code(device.verification_status()),
                        safety_number,
                        qr_payload_hex,
                    }
                })
            })
            .collect();

        let trusted_device_count = devices
            .iter()
            .filter(|device| matches!(device.state, VerificationStateCode::Verified))
            .count();
        let pending_device_count = devices.len().saturating_sub(trusted_device_count);

        VerificationWorkspaceView {
            trusted_device_count,
            pending_device_count,
            devices,
        }
    }

    fn security_label_for_device(&self, device_id: &str, forward_secrecy_active: bool) -> String {
        if let Some(device) = self.contacts.iter().find_map(|member| {
            member
                .devices()
                .find(|device| device.device_id().as_str() == device_id)
        }) {
            if device.is_verified() {
                if forward_secrecy_active {
                    "Verified device pair · Forward secrecy active".to_string()
                } else {
                    "Verified device pair".to_string()
                }
            } else {
                "Verification required before elevated trust".to_string()
            }
        } else {
            "Unknown trust state".to_string()
        }
    }

    fn sync_chat_labels(&mut self) {
        let updates: Vec<(String, String)> = self
            .chat_runtime_device_ids
            .iter()
            .map(|(chat_id, device_id)| {
                (
                    chat_id.clone(),
                    self.security_label_for_device(device_id, true),
                )
            })
            .collect();

        for (chat_id, security_label) in updates {
            if let Some(chat) = self.chats.iter_mut().find(|chat| chat.id == chat_id) {
                chat.presence_label = "secure session active".to_string();
                chat.security_label = security_label;
            }
        }
    }

    fn find_remote_device(&self, raw_device_id: &str) -> Option<(usize, DeviceId)> {
        self.contacts
            .iter()
            .enumerate()
            .find_map(|(index, member)| {
                member.devices().find_map(|device| {
                    if device.device_id().as_str() == raw_device_id {
                        Some((index, device.device_id().clone()))
                    } else {
                        None
                    }
                })
            })
    }
}

fn preview(body: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 72;

    let chars: Vec<char> = body.chars().collect();
    if chars.len() <= MAX_PREVIEW_CHARS {
        body.to_string()
    } else {
        let clipped: String = chars.into_iter().take(MAX_PREVIEW_CHARS - 3).collect();
        format!("{clipped}...")
    }
}

fn verification_state_code(status: &VerificationStatus) -> VerificationStateCode {
    match status {
        VerificationStatus::Pending => VerificationStateCode::Pending,
        VerificationStatus::Verified { .. } => VerificationStateCode::Verified,
    }
}

fn verification_method_code(status: &VerificationStatus) -> Option<VerificationMethodCode> {
    match status {
        VerificationStatus::Pending => None,
        VerificationStatus::Verified { method } => Some(match method {
            VerificationMethod::SafetyNumber => VerificationMethodCode::SafetyNumber,
            VerificationMethod::QrCode => VerificationMethodCode::QrCode,
        }),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(nibble_to_hex(byte >> 4));
        output.push(nibble_to_hex(byte & 0x0f));
    }
    output
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}

fn now_unix_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn timestamp_label(_unix_ms: i64) -> String {
    "now".to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        ClientState, PeerStateCode, VerificationAction, VerificationMethodCode,
        VerificationStateCode,
    };

    #[tokio::test]
    async fn snapshot_bootstrap_contains_chat_and_verification_data() {
        let state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");
        let snapshot = state.snapshot();

        assert_eq!(snapshot.chats.len(), 3);
        assert_eq!(snapshot.peers.len(), 3);
        assert!(snapshot.verification.trusted_device_count >= 2);
    }

    #[tokio::test]
    async fn send_message_appends_runtime_messages() {
        let mut state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");
        let before = state.snapshot().chats[0].messages.len();

        state
            .send_message("chat-bob", "UI smoke message")
            .await
            .expect("message should send");

        let snapshot = state.snapshot();
        let chat = snapshot
            .chats
            .iter()
            .find(|entry| entry.id == "chat-bob")
            .expect("chat should exist");
        assert!(chat.messages.len() >= before + 2);
        assert!(chat.summary.contains("QUIC") || chat.summary.contains("ACK"));
    }

    #[tokio::test]
    async fn verify_device_marks_pending_device_as_verified() {
        let mut state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");

        state
            .verify_device("daria-laptop", VerificationAction::Qr)
            .expect("verification should succeed");

        let snapshot = state.snapshot();
        let device = snapshot
            .verification
            .devices
            .iter()
            .find(|device| device.device_id == "daria-laptop")
            .expect("device should exist");

        assert!(matches!(device.state, VerificationStateCode::Verified));
        assert!(matches!(
            device.method,
            Some(VerificationMethodCode::QrCode)
        ));
    }

    #[tokio::test]
    async fn peer_refresh_reflects_runtime_presence() {
        let mut state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");

        state.refresh_peer_discovery();
        let snapshot = state.snapshot();

        assert!(matches!(snapshot.peers[0].state, PeerStateCode::Live));
        assert!(matches!(
            snapshot.peers[2].state,
            PeerStateCode::Reconnecting
        ));
    }
}
