use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use base64::Engine;
use crate::connection_manager::{ConnectionEvent, ConnectionManager, PeerTransportPresence};
use localmessenger_core::{
    Device, DeviceId, MemberId, MemberProfile, VerificationMethod, VerificationStatus,
};
use localmessenger_crypto::{
    IdentityKeyMaterial, IdentityKeyPair, LocalPrekeyStore, PrekeyStoreMaterial,
};
use localmessenger_discovery::{
    DiscoveredPeer, DiscoveryConfig, DiscoveryEvent, DiscoveryService, LocalPeerAnnouncement,
    PeerCapability,
};
use localmessenger_server_protocol::{
    ContactInvitePreview, DeviceContactInvite, DeviceRegistrationBundle, InvitePreview,
    contact_invite_preview, decode_contact_invite_server_certificate, encode_contact_invite_link,
    parse_contact_invite_link, sign_contact_invite,
};
use localmessenger_storage::{
    SqliteStorage, StorageKey, StoredLocalDeviceSecrets, StoredMessageKind, StoredPendingOutbound,
    StoredRemotePeerOffer, StoredTransportIdentity,
};
use rand_core::OsRng;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::media::{
    EncryptedBlob, MediaRoute, RELAY_MEDIA_MAX_BYTES, data_url, decrypt_blob, encrypt_blob,
    media_kind_for_mime, transfer_blob_over_quic,
};
use crate::relay_client::{
    RelayAuthStatus, RelayClient, RelayClientConfig, RelayServerStatus, TransportRoute,
    resolve_active_route,
};
use crate::runtime::{
    DirectChatRuntime, GroupChatRuntime, GroupRemoteMemberSpec, bootstrap_direct_chat_runtime,
    bootstrap_group_chat_runtime,
};
use localmessenger_transport::TransportIdentity;

pub type SharedClientState = Mutex<ClientState>;

pub struct ClientState {
    local_profile: MemberProfile,
    local_device_id: DeviceId,
    local_identity_material: IdentityKeyMaterial,
    local_prekey_store_material: PrekeyStoreMaterial,
    local_transport_identity: TransportIdentity,
    contacts: Vec<MemberProfile>,
    contact_runtimes: BTreeMap<String, DirectChatRuntime>,
    group_runtimes: BTreeMap<String, GroupChatRuntime>,
    chat_runtime_device_ids: BTreeMap<String, String>,
    discovered_peers: BTreeMap<String, DiscoveredPeer>,
    discovery_service: Option<DiscoveryService>,
    relay_client: Option<RelayClient>,
    relay_config: Option<RelayClientConfig>,
    preferred_routes: Vec<TransportRoute>,
    connection_manager: ConnectionManager,
    peer_presence: BTreeMap<String, PeerTransportPresence>,
    server_status: RelayServerStatus,
    auth_status: RelayAuthStatus,
    invite_preview: Option<InvitePreviewView>,
    contact_invite_preview: Option<ContactInvitePreviewView>,
    onboarding_status: String,
    updater_feed_url: Option<String>,
    updater_channel: String,
    updater_status: String,
    updater_last_checked_label: String,
    updater_can_auto_update: bool,
    tray_status: String,
    last_notification: String,
    chats: Vec<ChatThreadView>,
    message_counter: u64,
    pending_store: Option<SqliteStorage>,
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
    pub server_status: String,
    pub auth_status: String,
    pub active_route: String,
    pub notifications: NotificationCenterView,
    pub local_profile: LocalProfileView,
    pub chats: Vec<ChatThreadView>,
    pub peers: Vec<PeerView>,
    pub verification: VerificationWorkspaceView,
    pub onboarding: OnboardingView,
    pub updater: UpdaterView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportStatusView {
    pub discovery_mode: String,
    pub transport_mode: String,
    pub crypto_mode: String,
    pub storage_mode: String,
    pub server_status: String,
    pub auth_status: String,
    pub active_route: String,
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
pub struct OnboardingView {
    pub status_label: String,
    pub invite_preview: Option<InvitePreviewView>,
    pub contact_invite_preview: Option<ContactInvitePreviewView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterView {
    pub current_version: String,
    pub channel: String,
    pub status_label: String,
    pub last_checked_label: String,
    pub can_auto_update: bool,
    pub feed_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationCenterView {
    pub tray_label: String,
    pub unread_count: u32,
    pub last_event: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitePreviewView {
    pub invite_id: String,
    pub label: String,
    pub server_addr: String,
    pub server_name: String,
    pub expires_at_label: String,
    pub max_uses: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInvitePreviewView {
    pub member_id: String,
    pub device_id: String,
    pub display_name: String,
    pub server_addr: String,
    pub server_name: String,
    pub expires_at_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatThreadView {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub presence_label: String,
    pub presence_state: PresenceStateView,
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

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStateView {
    Online,
    Reconnecting,
    Offline,
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
    pub forwarded_from: Option<String>,
    pub reply_preview: Option<String>,
    pub reactions: Vec<String>,
    pub attachments: Vec<MessageAttachmentView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageAttachmentView {
    pub id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_label: String,
    pub transfer_route: String,
    pub status_label: String,
    pub preview_data_url: Option<String>,
    pub blob_id: Option<String>,
    pub upload_progress: f32,
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
    Seen,
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
    // ─────────────────────────────────────────────────────────────────────────
    // Production bootstrap — persistent identity in SQLite
    // ─────────────────────────────────────────────────────────────────────────

    /// Production entry point.  Called from `lib.rs` inside Tauri's `setup`
    /// hook where `AppHandle::path().app_data_dir()` is available.
    ///
    /// On first launch the function generates an Ed25519 identity keypair,
    /// saves it to an encrypted SQLite database and records a 32-byte
    /// bootstrap key alongside it.  On every subsequent launch it reloads the
    /// same key material so the user is always the same cryptographic identity.
    pub async fn bootstrap_persistent(app_data_dir: PathBuf) -> Result<Self, String> {
        use rand_core::RngCore;

        let mut rng = OsRng;

        fs::create_dir_all(&app_data_dir).map_err(|e| format!("create app-data dir: {e}"))?;

        // ── 1. Load or create the 32-byte bootstrap storage key ───────────
        let key_path = app_data_dir.join("bootstrap.key");
        let storage_key = load_or_create_storage_key(&key_path, &mut rng)?;

        // ── 2. Open encrypted SQLite ──────────────────────────────────────
        let db_url = format!(
            "sqlite://{}",
            app_data_dir.join("localmessenger.db").display()
        );
        let storage = SqliteStorage::open(&db_url, storage_key)
            .await
            .map_err(|e| format!("storage open failed: {e}"))?;

        // ── 3. Stable device-id persisted in a small text file ───────────
        let device_id_path = app_data_dir.join("device-id.txt");
        let device_id_str = if device_id_path.exists() {
            fs::read_to_string(&device_id_path)
                .map_err(|e| format!("read device-id.txt: {e}"))?
                .trim()
                .to_string()
        } else {
            let mut raw = [0u8; 8];
            rng.fill_bytes(&mut raw);
            let generated = format!(
                "device-{}",
                raw.iter().map(|b| format!("{b:02x}")).collect::<String>()
            );
            fs::write(&device_id_path, &generated)
                .map_err(|e| format!("write device-id.txt: {e}"))?;
            generated
        };
        let device_id = DeviceId::new(&device_id_str).map_err(|e| e.to_string())?;

        // ── 4. Resolve display name ───────────────────────────────────────
        let display_name = resolve_display_name();

        // ── 5. Load existing identity or create a new one ────────────────
        let (
            local_device,
            local_identity_material,
            local_prekey_store_material,
            local_transport_identity,
        ) = if let Some(mut secrets) = storage
            .local_device_secrets(&device_id)
            .await
            .map_err(|e| format!("load identity: {e}"))?
        {
            let material = secrets.identity_keypair().to_material();
            let transport_identity = match secrets.transport_identity.clone() {
                Some(stored) => TransportIdentity::from_der(
                    stored.server_name,
                    stored.certificate_der,
                    stored.private_key_der,
                ),
                None => {
                    let generated = TransportIdentity::generate(format!(
                        "{}.device.local",
                        device_id.as_str()
                    ))
                    .map_err(|e| e.to_string())?;
                    secrets.transport_identity = Some(StoredTransportIdentity {
                        server_name: generated.server_name.clone(),
                        certificate_der: generated.certificate_der.clone(),
                        private_key_der: generated.private_key_der.clone(),
                    });
                    storage
                        .store_local_device_secrets(&secrets)
                        .await
                        .map_err(|e| format!("store transport identity: {e}"))?;
                    generated
                }
            };
            let device = secrets.device.clone();
            let prekey_store_material = secrets.prekey_store_material.clone();
            (
                device,
                material,
                prekey_store_material,
                transport_identity,
            )
        } else {
            // First launch — generate identity, prekeys and save everything.
            let identity = IdentityKeyPair::generate(&mut rng);
            let material = identity.to_material();

            // member-id: keep only alphanumeric + dash chars (safe identifiers)
            let member_id_str: String = device_id_str
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            let member_id = MemberId::new(&member_id_str).map_err(|e| e.to_string())?;

            let prekey_seed = rng.next_u32() % 50_000;
            let prekeys = LocalPrekeyStore::generate(&mut rng, &identity, prekey_seed, 4, 10_000);
            let transport_identity = TransportIdentity::generate(format!(
                "{}.device.local",
                device_id.as_str()
            ))
            .map_err(|e| e.to_string())?;

            let device_label = format!("{} {}", display_name, system_device_name());
            let mut device = Device::from_identity_keypair(
                device_id.clone(),
                member_id,
                &device_label,
                &identity,
            )
            .map_err(|e| e.to_string())?;

            // Self-verify so the local device starts in a trusted state.
            let qr = device.qr_payload(None).map_err(|e| e.to_string())?;
            device
                .verify_with_qr_payload(&qr)
                .map_err(|e| e.to_string())?;

            let stored = StoredLocalDeviceSecrets::from_runtime(
                device.clone(),
                &identity,
                &prekeys,
                Some(StoredTransportIdentity {
                    server_name: transport_identity.server_name.clone(),
                    certificate_der: transport_identity.certificate_der.clone(),
                    private_key_der: transport_identity.private_key_der.clone(),
                }),
            )
            .map_err(|e| e.to_string())?;
            storage
                .store_local_device_secrets(&stored)
                .await
                .map_err(|e| format!("store identity: {e}"))?;

            (device, material, prekeys.to_material(), transport_identity)
        };

        // ── 6. Build local MemberProfile ──────────────────────────────────
        let member_id = local_device.owner_member_id().clone();
        let local_device_id = local_device.device_id().clone();
        let mut local_profile =
            MemberProfile::new(member_id.clone(), &display_name).map_err(|e| e.to_string())?;
        local_profile
            .add_device(local_device.clone())
            .map_err(|e| e.to_string())?;

        // ── 7. mDNS discovery ─────────────────────────────────────────────
        let local_announcement = LocalPeerAnnouncement::new(
            member_id.clone(),
            local_device_id.clone(),
            local_device.device_name(),
            46011,
            vec![PeerCapability::MessagingV1, PeerCapability::PresenceV1],
        )
        .map_err(|e| e.to_string())?;

        let discovery_service =
            match DiscoveryService::start(DiscoveryConfig::default(), local_announcement) {
                Ok(svc) => Some(svc),
                Err(e) => {
                    eprintln!("mDNS discovery failed to start: {e}");
                    None
                }
            };

        // ── 8. Relay client from environment variables ────────────────────
        let (relay_client, relay_config, preferred_routes, server_status, auth_status) =
            match RelayClientConfig::from_env(local_device_id.as_str())? {
                Some(config) => {
                    let auth_device_id =
                        DeviceId::new(config.auth_device_id.clone()).map_err(|e| e.to_string())?;
                    // Use local device as auth device; in a full implementation
                    // this would look up the matching device from storage.
                    let auth_device = if auth_device_id == local_device_id {
                        local_device.clone()
                    } else {
                        local_device.clone()
                    };
                    match RelayClient::connect(&config, &auth_device, &local_identity_material)
                        .await
                    {
                        Ok(bootstrap) => (
                            Some(bootstrap.client),
                            Some(config.clone()),
                            config.preferred_routes,
                            bootstrap.server_status,
                            bootstrap.auth_status,
                        ),
                        Err(_) => (
                            None,
                            Some(config.clone()),
                            config.preferred_routes,
                            RelayServerStatus::Failed,
                            RelayAuthStatus::Failed,
                        ),
                    }
                }
                None => (
                    None,
                    None,
                    vec![TransportRoute::DirectLan],
                    RelayServerStatus::Disabled,
                    RelayAuthStatus::Disabled,
                ),
            };

        // ── 9. Pending queue storage (same DB, separate storage key) ──────
        let pending_key_bytes: [u8; 32] = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(b"localmessenger/pending-store/v1");
            h.update(&local_identity_material.signing_secret_key);
            h.finalize().into()
        };
        let pending_store =
            match SqliteStorage::open(&db_url, StorageKey::from_bytes(pending_key_bytes)).await {
                Ok(s) => Some(s),
                Err(_) => None,
            };
        let stored_remote_offers = match &pending_store {
            Some(store) => store.list_remote_peer_offers().await.unwrap_or_default(),
            None => Vec::new(),
        };
        let mut connection_manager = ConnectionManager::new(
            local_device.clone(),
            local_identity_material.clone(),
            local_prekey_store_material.clone(),
            local_transport_identity.clone(),
            relay_client.clone(),
        );
        for stored_offer in &stored_remote_offers {
            let _ = connection_manager.upsert_contact_invite(stored_offer.invite.clone());
        }

        let mut state = Self {
            local_profile,
            local_device_id,
            local_identity_material,
            local_prekey_store_material,
            local_transport_identity,
            contacts: vec![],
            contact_runtimes: BTreeMap::new(),
            group_runtimes: BTreeMap::new(),
            chat_runtime_device_ids: BTreeMap::new(),
            discovered_peers: BTreeMap::new(),
            discovery_service,
            relay_client,
            relay_config,
            preferred_routes,
            connection_manager,
            peer_presence: BTreeMap::new(),
            server_status,
            auth_status,
            invite_preview: None,
            contact_invite_preview: None,
            onboarding_status: "Paste an invite link to join a relay server.".to_string(),
            updater_feed_url: std::env::var("LOCALMESSENGER_UPDATER_FEED").ok(),
            updater_channel: std::env::var("LOCALMESSENGER_UPDATER_CHANNEL")
                .unwrap_or_else(|_| "stable".to_string()),
            updater_status: "Updater is ready.".to_string(),
            updater_last_checked_label: "never".to_string(),
            updater_can_auto_update: false,
            tray_status: "Tray idle".to_string(),
            last_notification: "No notifications yet".to_string(),
            chats: vec![],
            message_counter: 0,
            pending_store,
        };
        for stored_offer in stored_remote_offers {
            state.register_remote_contact(&stored_offer.invite)?;
        }
        Ok(state)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Demo bootstrap — in-memory fake data used by unit tests
    // ─────────────────────────────────────────────────────────────────────────

    pub async fn bootstrap() -> Result<Self, String> {
        let mut rng = OsRng;

        let rimus_id = MemberId::new("rimus").map_err(|error| error.to_string())?;
        let mut rimus =
            MemberProfile::new(rimus_id.clone(), "Rimus").map_err(|error| error.to_string())?;

        let local_identity = IdentityKeyPair::generate(&mut rng);
        let local_identity_material = local_identity.to_material();
        let local_prekey_store_material =
            LocalPrekeyStore::generate(&mut rng, &local_identity, 11, 0, 0).to_material();
        let local_transport_identity =
            TransportIdentity::generate("rimus-laptop.device.local").map_err(|e| e.to_string())?;
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
            rimus_id.clone(),
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

        // ── Group chat runtime — full sender-key fan-out ───────────────────
        let lan_crew_runtime = bootstrap_group_chat_runtime(
            &local_reference,
            &local_identity_material,
            "lan-crew",
            "Rimus",
            vec![
                GroupRemoteMemberSpec {
                    member_id: "bob",
                    display_name: "Bob",
                    device_id: "grp-bob-phone",
                    device_name: "Bob Phone",
                    prekey_seed: 141,
                    reply_script: vec![
                        "Got it — group message delivered via sender-key fan-out!",
                        "Group QUIC path is solid.",
                    ],
                },
                GroupRemoteMemberSpec {
                    member_id: "carol",
                    display_name: "Carol",
                    device_id: "grp-carol-ws",
                    device_name: "Carol Workstation",
                    prekey_seed: 151,
                    reply_script: vec!["Carol here — group crypto is fully live!"],
                },
                GroupRemoteMemberSpec {
                    member_id: "daria",
                    display_name: "Daria",
                    device_id: "grp-daria-laptop",
                    device_name: "Daria Laptop",
                    prekey_seed: 161,
                    reply_script: vec![],
                },
            ],
        )
        .await?;
        let mut group_runtimes: BTreeMap<String, GroupChatRuntime> = BTreeMap::new();
        group_runtimes.insert("chat-lan-crew".to_string(), lan_crew_runtime);

        let chats = vec![
            ChatThreadView {
                id: "chat-bob".to_string(),
                title: "Bob".to_string(),
                summary: "Secure runtime session is active on QUIC loopback.".to_string(),
                presence_label: "secure session active".to_string(),
                presence_state: PresenceStateView::Online,
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
                        forwarded_from: None,
                        reply_preview: None,
                        reactions: vec!["ack".to_string()],
                        attachments: Vec::new(),
                    },
                    MessageView {
                        id: "m-2".to_string(),
                        author: "Rimus".to_string(),
                        body: "Good. I am wiring the desktop client to the secure backend."
                            .to_string(),
                        timestamp_label: "09:22".to_string(),
                        direction: MessageDirectionView::Outbound,
                        delivery_state: DeliveryStateView::Seen,
                        forwarded_from: None,
                        reply_preview: Some(
                            "I am back on the local runtime. QUIC path is stable now.".to_string(),
                        ),
                        reactions: Vec::new(),
                        attachments: vec![sample_voice_attachment()],
                    },
                ],
            },
            ChatThreadView {
                id: "chat-lan-crew".to_string(),
                title: "LAN Crew".to_string(),
                summary: "Sender-key fan-out live · all pairwise sessions active.".to_string(),
                presence_label: "group session active · 3 members".to_string(),
                presence_state: PresenceStateView::Online,
                unread_count: 0,
                security_label: "Group sender key epoch 0 · verified".to_string(),
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
                        body:
                            "Group session bootstrapped — sender keys exchanged with all 3 members."
                                .to_string(),
                        timestamp_label: "08:41".to_string(),
                        direction: MessageDirectionView::System,
                        delivery_state: DeliveryStateView::Delivered,
                        forwarded_from: None,
                        reply_preview: None,
                        reactions: Vec::new(),
                        attachments: Vec::new(),
                    },
                    MessageView {
                        id: "g-2".to_string(),
                        author: "Carol".to_string(),
                        body: "Group crypto is live — type a message to test the fan-out."
                            .to_string(),
                        timestamp_label: "08:45".to_string(),
                        direction: MessageDirectionView::Inbound,
                        delivery_state: DeliveryStateView::Delivered,
                        forwarded_from: None,
                        reply_preview: None,
                        reactions: Vec::new(),
                        attachments: Vec::new(),
                    },
                ],
            },
            ChatThreadView {
                id: "chat-carol".to_string(),
                title: "Carol".to_string(),
                summary: "Attachments stay locked until the verification workspace is green."
                    .to_string(),
                presence_label: "secure session active".to_string(),
                presence_state: PresenceStateView::Online,
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
                    forwarded_from: Some("LAN Crew".to_string()),
                    reply_preview: None,
                    reactions: Vec::new(),
                    attachments: vec![sample_photo_attachment(), sample_pdf_attachment()],
                }],
            },
        ];

        let chat_runtime_device_ids = BTreeMap::from([
            ("chat-bob".to_string(), "bob-phone".to_string()),
            ("chat-carol".to_string(), "carol-workstation".to_string()),
        ]);

        let (relay_client, relay_config, preferred_routes, server_status, auth_status) =
            match RelayClientConfig::from_env(local_device_id.as_str())? {
                Some(config) => {
                    let auth_device_id = DeviceId::new(config.auth_device_id.clone())
                        .map_err(|error| error.to_string())?;
                    let auth_device = rimus
                        .device(&auth_device_id)
                        .cloned()
                        .ok_or_else(|| "relay auth device is missing".to_string())?;
                    match RelayClient::connect(&config, &auth_device, &local_identity_material)
                        .await
                    {
                        Ok(bootstrap) => (
                            Some(bootstrap.client),
                            Some(config.clone()),
                            config.preferred_routes,
                            bootstrap.server_status,
                            bootstrap.auth_status,
                        ),
                        Err(_) => (
                            None,
                            Some(config.clone()),
                            config.preferred_routes,
                            RelayServerStatus::Failed,
                            RelayAuthStatus::Failed,
                        ),
                    }
                }
                None => (
                    None,
                    None,
                    vec![TransportRoute::DirectLan],
                    RelayServerStatus::Disabled,
                    RelayAuthStatus::Disabled,
                ),
            };

        let local_announcement = LocalPeerAnnouncement::new(
            rimus_id.clone(),
            local_device_id.clone(),
            "Rimus Laptop",
            46011,
            vec![PeerCapability::MessagingV1, PeerCapability::PresenceV1],
        )
        .map_err(|error| error.to_string())?;

        let discovery_service =
            match DiscoveryService::start(DiscoveryConfig::default(), local_announcement) {
                Ok(service) => Some(service),
                Err(error) => {
                    eprintln!("mDNS discovery failed to start: {error}");
                    None
                }
            };

        // Open a durable pending-queue store keyed to the local device identity.
        // The storage key is deterministically derived from the local identity so
        // the same database can be decrypted across restarts.
        let storage_key_bytes: [u8; 32] = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(b"localmessenger/pending-store/v1");
            h.update(&local_identity_material.signing_secret_key);
            h.finalize().into()
        };
        let pending_store =
            match SqliteStorage::open("sqlite::memory:", StorageKey::from_bytes(storage_key_bytes))
                .await
            {
                Ok(store) => Some(store),
                Err(_) => None,
            };
        let connection_manager = ConnectionManager::new(
            local_reference.clone(),
            local_identity_material.clone(),
            local_prekey_store_material.clone(),
            local_transport_identity.clone(),
            relay_client.clone(),
        );

        Ok(Self {
            local_profile:          rimus,
            local_device_id,
            local_identity_material,
            local_prekey_store_material,
            local_transport_identity,
            contacts,
            contact_runtimes,
            group_runtimes,
            chat_runtime_device_ids,
            discovered_peers: BTreeMap::new(),
            discovery_service,
            relay_client,
            relay_config,
            preferred_routes,
            connection_manager,
            peer_presence: BTreeMap::new(),
            server_status,
            auth_status,
            invite_preview: None,
            contact_invite_preview: None,
            onboarding_status: "Paste an invite link to join a relay.".to_string(),
            updater_feed_url: std::env::var("LOCALMESSENGER_UPDATER_FEED").ok(),
            updater_channel: std::env::var("LOCALMESSENGER_UPDATER_CHANNEL")
                .unwrap_or_else(|_| "stable".to_string()),
            updater_status:
                "Updater artifacts are enabled for release builds. Runtime auto-install is disabled in this desktop shell."
                    .to_string(),
            updater_last_checked_label: "never".to_string(),
            updater_can_auto_update: false,
            tray_status: "Tray idle".to_string(),
            last_notification: "No new notifications".to_string(),
            chats,
            message_counter: 100,
            pending_store,
        })
    }

    pub fn snapshot(&self) -> ClientSnapshot {
        let active_device = self
            .local_profile
            .device(&self.local_device_id)
            .map(|device| device.device_name().to_string())
            .unwrap_or_else(|| "Unknown Device".to_string());
        let active_route =
            resolve_active_route(&self.preferred_routes, self.relay_client.is_some());

        ClientSnapshot {
            transport_status: TransportStatusView {
                discovery_mode: if self.discovery_service.is_some() {
                    format!(
                        "mDNS discovery active ({} peer(s) found)",
                        self.discovered_peers.len()
                    )
                } else {
                    "mDNS discovery disabled".to_string()
                },
                transport_mode: "QUIC transport with relay fallback routing".to_string(),
                crypto_mode: "X3DH bootstrap + Double Ratchet".to_string(),
                storage_mode: "Encrypted local state + relay blob storage for media".to_string(),
                server_status: self.server_status.label().to_string(),
                auth_status: self.auth_status.label().to_string(),
                active_route: active_route.label().to_string(),
            },
            server_status: self.server_status.label().to_string(),
            auth_status: self.auth_status.label().to_string(),
            active_route: active_route.label().to_string(),
            notifications: NotificationCenterView {
                tray_label: self.tray_status.clone(),
                unread_count: self.total_unread_count(),
                last_event: self.last_notification.clone(),
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
            onboarding: OnboardingView {
                status_label: self.onboarding_status.clone(),
                invite_preview: self.invite_preview.clone(),
                contact_invite_preview: self.contact_invite_preview.clone(),
            },
            updater: UpdaterView {
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                channel: self.updater_channel.clone(),
                status_label: self.updater_status.clone(),
                last_checked_label: self.updater_last_checked_label.clone(),
                can_auto_update: self.updater_can_auto_update,
                feed_url: self.updater_feed_url.clone(),
            },
        }
    }

    pub fn poll_background_state(&mut self) {
        self.drain_connection_events();
        self.sync_chat_labels();
    }

    pub fn refresh_peer_discovery(&mut self) {
        self.drain_connection_events();

        // Drain mDNS discovery events
        if let Some(service) = &self.discovery_service {
            let mut receiver = service.subscribe();
            loop {
                match receiver.try_recv() {
                    Ok(DiscoveryEvent::PeerAdded(peer)) | Ok(DiscoveryEvent::PeerUpdated(peer)) => {
                        if self.connection_manager.has_contact(peer.device_id.as_str()) {
                            self.peer_presence.insert(
                                peer.device_id.to_string(),
                                PeerTransportPresence::LanOnline,
                            );
                        }
                        self.discovered_peers
                            .insert(peer.device_id.to_string(), peer);
                    }
                    Ok(DiscoveryEvent::PeerExpired(peer)) => {
                        self.discovered_peers.remove(peer.device_id.as_str());
                        if self.connection_manager.has_contact(peer.device_id.as_str()) {
                            self.peer_presence.insert(
                                peer.device_id.to_string(),
                                PeerTransportPresence::OfflineButQueueable,
                            );
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        // Existing relay health check
        if let Some(relay_client) = &self.relay_client {
            let relay_client = relay_client.clone();
            tokio::spawn(async move {
                let _ = relay_client.health_check().await;
            });
        }
        self.sync_chat_labels();
        let unread = self.total_unread_count();
        self.tray_status = if unread == 0 {
            "Tray idle".to_string()
        } else {
            format!("{unread} unread")
        };
    }

    pub async fn start_chat_with_peer(&mut self, device_id: &str) -> Result<(), String> {
        // Check if chat already exists
        if self
            .chat_runtime_device_ids
            .values()
            .any(|v| v == device_id)
        {
            return Err("Chat with this device already exists".to_string());
        }

        let discovered_peer = self
            .discovered_peers
            .get(device_id)
            .cloned()
            .ok_or_else(|| {
                format!("Device '{device_id}' not found in discovered peers. Refresh peers first.")
            })?;

        let _remote_addr = discovered_peer.socket_address.ok_or_else(|| {
            format!(
                "Discovered peer '{}' has no network address",
                discovered_peer.device_name
            )
        })?;

        // Bootstrap a runtime using InMemoryFrameChannel for now.
        // TODO: Replace with real QUIC TransportConnection to remote_addr
        let remote_member_id = discovered_peer.member_id.as_str().to_string();
        let remote_device_name = discovered_peer.device_name.clone();
        let remote_capabilities = discovered_peer.capabilities.clone();
        let chat_id = format!("chat-{}", device_id);

        let bootstrap = bootstrap_direct_chat_runtime(
            &self
                .local_profile
                .device(&self.local_device_id)
                .cloned()
                .ok_or_else(|| "local device missing".to_string())?,
            &self.local_identity_material,
            &remote_member_id,
            &remote_device_name,
            device_id,
            &remote_device_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos(),
            remote_capabilities,
            vec!["Hello! Connection established."],
        )
        .await?;

        let remote_device_id = bootstrap.runtime.remote_device_id().clone();

        // Add contact
        self.contacts.push(bootstrap.member);

        // Add runtime
        self.contact_runtimes
            .insert(remote_device_id.to_string(), bootstrap.runtime);

        // Create chat thread
        self.message_counter = self.message_counter.saturating_add(1);
        self.chats.push(ChatThreadView {
            id: chat_id.clone(),
            title: remote_device_name.clone(),
            summary: format!("Secure session with {}", remote_device_name),
            presence_label: "secure session active".to_string(),
            presence_state: PresenceStateView::Online,
            unread_count: 0,
            security_label: "E2EE session established".to_string(),
            kind: ChatKindView::Direct,
            participants: vec![
                self.local_profile.display_name().to_string(),
                remote_device_name,
            ],
            messages: Vec::new(),
        });

        self.chat_runtime_device_ids
            .insert(chat_id, remote_device_id.to_string());

        self.push_notification(format!("Started chat with peer {device_id}"));
        Ok(())
    }

    pub async fn send_message(
        &mut self,
        chat_id: &str,
        body: &str,
        reply_to_message_id: Option<&str>,
    ) -> Result<(), String> {
        self.drain_connection_events();
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Err("message body cannot be empty".to_string());
        }
        let reply_preview = self.reply_preview_for(chat_id, reply_to_message_id)?;
        let preferred_route =
            resolve_active_route(&self.preferred_routes, self.relay_client.is_some());

        let chat = self
            .chats
            .iter()
            .find(|chat| chat.id == chat_id)
            .ok_or_else(|| format!("chat '{chat_id}' not found"))?;
        if matches!(chat.kind, ChatKindView::Group) {
            // ── Group message path: encrypt + fan-out via GroupChatRuntime ──
            self.message_counter = self.message_counter.saturating_add(1);
            let outbound_message_id = format!("local-{}", self.message_counter);
            let sent_at_unix_ms = now_unix_ms();
            let local_author = self.local_profile.display_name().to_string();

            let outcome = {
                let group_runtime = self
                    .group_runtimes
                    .get_mut(chat_id)
                    .ok_or_else(|| format!("group runtime for '{chat_id}' not found"))?;
                group_runtime
                    .send_text(
                        chat_id,
                        outbound_message_id.clone(),
                        sent_at_unix_ms,
                        trimmed.to_string(),
                    )
                    .await?
            };

            let chat = self
                .chats
                .iter_mut()
                .find(|c| c.id == chat_id)
                .ok_or_else(|| format!("chat '{chat_id}' not found"))?;

            chat.messages.push(MessageView {
                id: outbound_message_id,
                author: local_author,
                body: trimmed.to_string(),
                timestamp_label: timestamp_label(sent_at_unix_ms),
                direction: MessageDirectionView::Outbound,
                delivery_state: DeliveryStateView::Delivered,
                forwarded_from: None,
                reply_preview,
                reactions: Vec::new(),
                attachments: Vec::new(),
            });

            for inbound in &outcome.inbound_messages {
                chat.messages.push(MessageView {
                    id: inbound.message_id.clone(),
                    author: inbound.author.clone(),
                    body: inbound.body.clone(),
                    timestamp_label: timestamp_label(inbound.sent_at_unix_ms),
                    direction: MessageDirectionView::Inbound,
                    delivery_state: DeliveryStateView::Delivered,
                    forwarded_from: None,
                    reply_preview: None,
                    reactions: Vec::new(),
                    attachments: Vec::new(),
                });
            }

            if let Some(last_msg) = chat.messages.last() {
                chat.summary = preview(&last_msg.body);
            }
            chat.presence_label = format!(
                "group session active · {} of {} members reached",
                outcome.members_reached, outcome.members_reached,
            );
            chat.presence_state = PresenceStateView::Online;
            let inbound_count = outcome.inbound_messages.len() as u32;
            if inbound_count > 0 {
                chat.unread_count = chat.unread_count.saturating_add(inbound_count);
                let chat_title = chat.title.clone();
                self.push_notification(format!("New messages in {chat_title}"));
            }
            return Ok(());
        }

        let remote_device_id = self
            .chat_runtime_device_ids
            .get(chat_id)
            .cloned()
            .ok_or_else(|| format!("chat '{chat_id}' is not bound to a runtime session"))?;
        self.message_counter = self.message_counter.saturating_add(1);
        let outbound_message_id = format!("local-{}", self.message_counter);
        let sent_at_unix_ms = now_unix_ms();

        let remote_author = self
            .remote_display_name_for_device(&remote_device_id)
            .unwrap_or_else(|| "Remote peer".to_string());
        let (outbound_acknowledged, forward_secrecy_active, immediate_inbound_messages) =
            if self.contact_runtimes.contains_key(&remote_device_id) {
                let outcome = {
                    let runtime = self
                        .contact_runtimes
                        .get_mut(&remote_device_id)
                        .ok_or_else(|| format!("runtime for device '{remote_device_id}' is missing"))?;
                    runtime
                        .send_text(
                            chat_id,
                            outbound_message_id.clone(),
                            sent_at_unix_ms,
                            trimmed.to_string(),
                        )
                        .await?
                };

                if let Some(store) = &self.pending_store {
                    let snap = {
                        let runtime = self
                            .contact_runtimes
                            .get(&remote_device_id)
                            .ok_or_else(|| format!("runtime for device '{remote_device_id}' is missing"))?;
                        runtime.engine_snapshot()
                    };
                    for msg in &snap.pending_messages {
                        if let Ok(entry) = StoredPendingOutbound::new(
                            remote_device_id.as_str(),
                            msg.delivery_order(),
                            msg.message_id(),
                            msg.conversation_id(),
                            msg.sent_at_unix_ms(),
                            StoredMessageKind::Text,
                            msg.body().to_vec(),
                            msg.attempt_count(),
                        ) {
                            let _ = store.upsert_pending_outbound(&entry).await;
                        }
                    }
                    if outcome.outbound_acknowledged {
                        let _ = store
                            .remove_pending_outbound(remote_device_id.as_str(), &outbound_message_id)
                            .await;
                    }
                }

                (
                    outcome.outbound_acknowledged,
                    outcome.forward_secrecy_active,
                    outcome.inbound_messages,
                )
            } else if self.connection_manager.has_contact(&remote_device_id) {
                let outcome = self
                    .connection_manager
                    .send_text(
                        &remote_device_id,
                        chat_id.to_string(),
                        outbound_message_id.clone(),
                        sent_at_unix_ms,
                        trimmed.to_string(),
                    )
                    .await?;
                self.drain_connection_events();
                (
                    outcome.outbound_acknowledged,
                    outcome.forward_secrecy_active,
                    Vec::new(),
                )
            } else {
                return Err(format!("runtime for device '{remote_device_id}' is missing"));
            };

        let security_label =
            self.security_label_for_device(&remote_device_id, forward_secrecy_active);

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
            delivery_state: if !immediate_inbound_messages.is_empty() {
                DeliveryStateView::Seen
            } else if outbound_acknowledged {
                DeliveryStateView::Delivered
            } else {
                DeliveryStateView::Sent
            },
            forwarded_from: None,
            reply_preview,
            reactions: Vec::new(),
            attachments: Vec::new(),
        });

        for inbound in immediate_inbound_messages {
            chat.messages.push(MessageView {
                id: inbound.message_id,
                author: remote_author.clone(),
                body: inbound.body,
                timestamp_label: timestamp_label(inbound.sent_at_unix_ms),
                direction: MessageDirectionView::Inbound,
                delivery_state: DeliveryStateView::Delivered,
                forwarded_from: None,
                reply_preview: Some(trimmed.to_string()),
                reactions: Vec::new(),
                attachments: Vec::new(),
            });
        }

        if let Some(last_message) = chat.messages.last() {
            chat.summary = preview(&last_message.body);
        }
        let chat_title = chat.title.clone();
        let inbound_count = chat
            .messages
            .iter()
            .rev()
            .take_while(|message| matches!(message.direction, MessageDirectionView::Inbound))
            .count() as u32;
        if inbound_count > 0 {
            chat.unread_count = chat.unread_count.saturating_add(inbound_count);
        } else {
            chat.unread_count = 0;
        }
        chat.presence_label = match preferred_route {
            TransportRoute::ServerRelay => {
                "relay preferred, direct LAN fallback active".to_string()
            }
            TransportRoute::DirectLan => "secure session active · direct LAN".to_string(),
        };
        chat.presence_state = PresenceStateView::Online;
        chat.security_label = security_label;
        if inbound_count > 0 {
            self.push_notification(format!("{remote_author} replied in {chat_title}"));
        }

        Ok(())
    }

    pub async fn send_media(
        &mut self,
        chat_id: &str,
        file_name: &str,
        mime_type: &str,
        bytes_base64: &str,
        reply_to_message_id: Option<&str>,
    ) -> Result<(), String> {
        let trimmed_name = file_name.trim();
        let trimmed_mime = mime_type.trim();
        if trimmed_name.is_empty() {
            return Err("file name cannot be empty".to_string());
        }
        if trimmed_mime.is_empty() {
            return Err("mime type cannot be empty".to_string());
        }

        let plaintext = base64::engine::general_purpose::STANDARD
            .decode(bytes_base64)
            .map_err(|error| format!("invalid base64 payload: {error}"))?;
        if plaintext.is_empty() {
            return Err("media payload cannot be empty".to_string());
        }
        let reply_preview = self.reply_preview_for(chat_id, reply_to_message_id)?;

        let _chat = self
            .chats
            .iter()
            .find(|chat| chat.id == chat_id)
            .ok_or_else(|| format!("chat '{chat_id}' not found"))?;
        let route = if plaintext.len() <= RELAY_MEDIA_MAX_BYTES && self.relay_client.is_some() {
            MediaRoute::RelayBlobStore
        } else {
            MediaRoute::DirectQuic
        };
        let encrypted = encrypt_blob(&plaintext)?;
        let attachment_id = format!("att-{}", self.message_counter.saturating_add(1));
        let (status_label, blob_id) = match route {
            MediaRoute::RelayBlobStore => {
                let relay_client = self
                    .relay_client
                    .clone()
                    .ok_or_else(|| "relay client is unavailable".to_string())?;
                let uploaded = relay_client
                    .upload_blob(
                        trimmed_name,
                        trimmed_mime,
                        media_kind_for_mime(trimmed_mime),
                        plaintext.len() as u64,
                        encrypted.ciphertext.clone(),
                        encrypted.sha256_hex.clone(),
                    )
                    .await?;
                let downloaded = relay_client
                    .download_blob(&uploaded.metadata.blob_id)
                    .await?;
                let round_trip = EncryptedBlob {
                    ciphertext: downloaded.ciphertext,
                    key: encrypted.key,
                    nonce: encrypted.nonce,
                    sha256_hex: downloaded.metadata.sha256_hex.clone(),
                };
                let restored = decrypt_blob(&round_trip)?;
                if restored != plaintext {
                    return Err("relay blob round-trip integrity check failed".to_string());
                }
                (
                    format!("encrypted relay blob ready · {}", uploaded.metadata.blob_id),
                    Some(uploaded.metadata.blob_id),
                )
            }
            MediaRoute::DirectQuic => {
                let receipt = transfer_blob_over_quic(
                    trimmed_name,
                    trimmed_mime,
                    encrypted.ciphertext.clone(),
                    encrypted.sha256_hex.clone(),
                )
                .await?;
                (
                    format!(
                        "direct QUIC handoff complete · {} · {} bytes · {}",
                        receipt.transfer_id,
                        receipt.transferred_bytes,
                        &receipt.sha256_hex[..12]
                    ),
                    None,
                )
            }
        };

        self.message_counter = self.message_counter.saturating_add(1);
        let message_id = format!("local-{}", self.message_counter);
        let sent_at_unix_ms = now_unix_ms();
        let attachment = MessageAttachmentView {
            id: attachment_id,
            file_name: trimmed_name.to_string(),
            mime_type: trimmed_mime.to_string(),
            size_label: byte_count_label(plaintext.len()),
            transfer_route: route.label().to_string(),
            status_label,
            preview_data_url: if trimmed_mime.starts_with("image/")
                || trimmed_mime.starts_with("audio/")
                || trimmed_mime == "application/pdf"
            {
                Some(data_url(trimmed_mime, &plaintext))
            } else {
                None
            },
            blob_id,
            upload_progress: 1.0,
        };

        let chat = self
            .chats
            .iter_mut()
            .find(|chat| chat.id == chat_id)
            .ok_or_else(|| format!("chat '{chat_id}' not found"))?;
        chat.messages.push(MessageView {
            id: message_id,
            author: self.local_profile.display_name().to_string(),
            body: attachment_body(trimmed_name, trimmed_mime),
            timestamp_label: timestamp_label(sent_at_unix_ms),
            direction: MessageDirectionView::Outbound,
            delivery_state: DeliveryStateView::Seen,
            forwarded_from: None,
            reply_preview,
            reactions: Vec::new(),
            attachments: vec![attachment],
        });
        if let Some(last_message) = chat.messages.last() {
            chat.summary = preview(&last_message.body);
        }
        let chat_title = chat.title.clone();
        chat.unread_count = 0;
        chat.presence_label = match route {
            MediaRoute::RelayBlobStore => "relay blob storage active for small media".to_string(),
            MediaRoute::DirectQuic => "direct QUIC file lane active".to_string(),
        };
        chat.presence_state = PresenceStateView::Online;
        self.push_notification(format!("Shared media in {chat_title}"));
        Ok(())
    }

    pub fn toggle_reaction(
        &mut self,
        chat_id: &str,
        message_id: &str,
        reaction: &str,
    ) -> Result<(), String> {
        let trimmed = reaction.trim();
        if trimmed.is_empty() {
            return Err("reaction cannot be empty".to_string());
        }

        let chat = self
            .chats
            .iter_mut()
            .find(|chat| chat.id == chat_id)
            .ok_or_else(|| format!("chat '{chat_id}' not found"))?;
        let message = chat
            .messages
            .iter_mut()
            .find(|message| message.id == message_id)
            .ok_or_else(|| format!("message '{message_id}' was not found"))?;
        if let Some(position) = message.reactions.iter().position(|entry| entry == trimmed) {
            message.reactions.remove(position);
        } else {
            message.reactions.push(trimmed.to_string());
        }
        let chat_title = chat.title.clone();
        self.push_notification(format!("Reaction updated in {chat_title}"));
        Ok(())
    }

    pub fn forward_message(
        &mut self,
        source_chat_id: &str,
        target_chat_id: &str,
        message_id: &str,
    ) -> Result<(), String> {
        let source_chat = self
            .chats
            .iter()
            .find(|chat| chat.id == source_chat_id)
            .cloned()
            .ok_or_else(|| format!("chat '{source_chat_id}' not found"))?;
        let source_message = source_chat
            .messages
            .iter()
            .find(|message| message.id == message_id)
            .cloned()
            .ok_or_else(|| format!("message '{message_id}' was not found"))?;

        let target_chat = self
            .chats
            .iter()
            .find(|chat| chat.id == target_chat_id)
            .ok_or_else(|| format!("chat '{target_chat_id}' not found"))?;
        if matches!(target_chat.kind, ChatKindView::Group) {
            return Err("group forwarding is still staged in the desktop client".to_string());
        }

        self.message_counter = self.message_counter.saturating_add(1);
        let forwarded_id = format!("local-{}", self.message_counter);
        let sent_at_unix_ms = now_unix_ms();
        let chat_title = {
            let target_chat = self
                .chats
                .iter_mut()
                .find(|chat| chat.id == target_chat_id)
                .ok_or_else(|| format!("chat '{target_chat_id}' not found"))?;
            target_chat.messages.push(MessageView {
                id: forwarded_id,
                author: self.local_profile.display_name().to_string(),
                body: source_message.body.clone(),
                timestamp_label: timestamp_label(sent_at_unix_ms),
                direction: MessageDirectionView::Outbound,
                delivery_state: DeliveryStateView::Delivered,
                forwarded_from: Some(source_chat.title.clone()),
                reply_preview: source_message.reply_preview.clone(),
                reactions: Vec::new(),
                attachments: source_message.attachments.clone(),
            });
            if let Some(last_message) = target_chat.messages.last() {
                target_chat.summary = preview(&last_message.body);
            }
            target_chat.unread_count = 0;
            target_chat.presence_label = "secure session active · forwarded locally".to_string();
            target_chat.presence_state = PresenceStateView::Online;
            target_chat.title.clone()
        };
        self.push_notification(format!("Forwarded message to {chat_title}"));
        Ok(())
    }

    pub fn check_for_updates(&mut self) -> Result<(), String> {
        self.updater_last_checked_label = "just now".to_string();
        self.updater_status = match &self.updater_feed_url {
            Some(feed_url) => format!(
                "Signed updater feed configured at {feed_url}, but runtime install is disabled until the packaged app enables the Tauri updater plugin."
            ),
            None => "No updater feed configured. Release bundles can still produce signed updater artifacts.".to_string(),
        };
        self.push_notification("Update status refreshed".to_string());
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

    pub fn export_device_registration(&self, path: &str) -> Result<(), String> {
        let local_device = self
            .local_profile
            .device(&self.local_device_id)
            .ok_or_else(|| "local active device is missing".to_string())?;
        let bundle = DeviceRegistrationBundle::new(
            local_device.owner_member_id(),
            local_device.device_id(),
            local_device.device_name(),
            local_device.identity_keys().signing_public_key,
        );
        let json = serde_json::to_vec_pretty(&bundle).map_err(|error| error.to_string())?;
        fs::write(path, json).map_err(|error| error.to_string())
    }

    pub fn preview_invite(&mut self, invite_link: &str) -> Result<(), String> {
        let preview = RelayClient::preview_invite(invite_link)?;
        self.invite_preview = Some(invite_preview_view(&preview));
        self.contact_invite_preview = None;
        self.onboarding_status = format!(
            "Invite ready for {} at {}.",
            preview.label, preview.server_addr
        );
        self.push_notification(format!("Invite previewed for {}", preview.server_name));
        Ok(())
    }

    pub fn create_contact_invite(&self) -> Result<String, String> {
        let relay_config = self
            .relay_config
            .as_ref()
            .ok_or_else(|| "join a relay before creating a contact invite".to_string())?;
        let local_device = self
            .local_profile
            .device(&self.local_device_id)
            .ok_or_else(|| "local active device is missing".to_string())?;
        let identity = IdentityKeyPair::from_material(&self.local_identity_material);
        let prekey_store = LocalPrekeyStore::from_material(self.local_prekey_store_material.clone())
            .map_err(|error| error.to_string())?;
        let mut public_bundle = prekey_store.public_bundle();
        public_bundle.one_time_prekeys.clear();
        let mut invite = DeviceContactInvite {
            version: localmessenger_server_protocol::SERVER_PROTOCOL_VERSION,
            member_id: local_device.owner_member_id().to_string(),
            device_id: local_device.device_id().to_string(),
            display_name: self.local_profile.display_name().to_string(),
            server_addr: relay_config.server_addr.to_string(),
            server_name: relay_config.server_name.clone(),
            server_certificate_der_base64: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(&relay_config.trusted_server_certificate_der),
            device_transport_certificate_der_base64:
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(&self.local_transport_identity.certificate_der),
            identity_keys: local_device.identity_keys().clone(),
            prekey_bundle: public_bundle,
            issued_at_unix_ms: now_unix_ms(),
            expires_at_unix_ms: now_unix_ms().saturating_add(7 * 24 * 60 * 60 * 1000),
            signature: Vec::new(),
        };
        sign_contact_invite(&identity, &mut invite)?;
        encode_contact_invite_link(&invite)
    }

    pub fn preview_contact_invite(&mut self, invite_link: &str) -> Result<(), String> {
        let invite = parse_contact_invite_link(invite_link)?;
        let preview = contact_invite_preview(&invite);
        self.invite_preview = None;
        self.contact_invite_preview = Some(contact_invite_preview_view(&preview));
        self.onboarding_status = format!(
            "Contact invite ready for {} on {}.",
            preview.display_name, preview.server_addr
        );
        self.push_notification(format!("Contact invite previewed for {}", preview.display_name));
        Ok(())
    }

    pub async fn accept_invite(&mut self, invite_link: &str) -> Result<(), String> {
        let local_device = self
            .local_profile
            .device(&self.local_device_id)
            .cloned()
            .ok_or_else(|| "local active device is missing".to_string())?;
        let registration = DeviceRegistrationBundle::new(
            local_device.owner_member_id(),
            local_device.device_id(),
            local_device.device_name(),
            local_device.identity_keys().signing_public_key,
        );
        let join = RelayClient::join_with_invite(invite_link, registration).await?;
        let preferred_routes = vec![TransportRoute::ServerRelay, TransportRoute::DirectLan];
        let config = RelayClientConfig::from_join_accepted(
            &join,
            local_device.device_id().to_string(),
            preferred_routes.clone(),
        )?;
        let bootstrap =
            RelayClient::connect(&config, &local_device, &self.local_identity_material).await?;

        self.relay_client = Some(bootstrap.client);
        self.relay_config = Some(config);
        self.preferred_routes = preferred_routes;
        self.server_status = bootstrap.server_status;
        self.auth_status = bootstrap.auth_status;
        self.connection_manager
            .set_relay_client(self.relay_client.clone());
        self.invite_preview = Some(InvitePreviewView {
            invite_id: join.invite_id.clone(),
            label: "Joined relay".to_string(),
            server_addr: join.server_addr.clone(),
            server_name: join.server_name.clone(),
            expires_at_label: "active".to_string(),
            max_uses: 0,
        });
        self.onboarding_status =
            format!("Joined relay {} as {}.", join.server_addr, join.device_id);
        self.push_notification(format!("Relay joined: {}", join.server_addr));
        Ok(())
    }

    pub async fn accept_contact_invite(&mut self, invite_link: &str) -> Result<(), String> {
        let invite = parse_contact_invite_link(invite_link)?;
        if let Some(relay_config) = &self.relay_config {
            let remote_server_certificate = decode_contact_invite_server_certificate(&invite)?;
            if relay_config.server_addr.to_string() != invite.server_addr
                || relay_config.server_name != invite.server_name
                || relay_config.trusted_server_certificate_der != remote_server_certificate
            {
                return Err(
                    "contact invite points to a different relay than the active desktop session"
                        .to_string(),
                );
            }
        }

        if let Some(store) = &self.pending_store {
            let _ = store
                .upsert_remote_peer_offer(&StoredRemotePeerOffer {
                    invite: invite.clone(),
                })
                .await;
        }

        self.connection_manager
            .upsert_contact_invite(invite.clone())?;
        self.register_remote_contact(&invite)?;
        self.contact_invite_preview = Some(contact_invite_preview_view(&contact_invite_preview(
            &invite,
        )));
        self.onboarding_status = format!(
            "Accepted contact invite from {} ({}).",
            invite.display_name, invite.device_id
        );
        self.push_notification(format!("Contact added: {}", invite.display_name));
        Ok(())
    }

    fn register_remote_contact(&mut self, invite: &DeviceContactInvite) -> Result<(), String> {
        let member_id = MemberId::new(invite.member_id.clone()).map_err(|error| error.to_string())?;
        let device_id = DeviceId::new(invite.device_id.clone()).map_err(|error| error.to_string())?;
        let device = Device::new(
            device_id.clone(),
            member_id.clone(),
            invite.display_name.clone(),
            invite.identity_keys.clone(),
        )
        .map_err(|error| error.to_string())?;

        if let Some(member) = self
            .contacts
            .iter_mut()
            .find(|member| member.member_id() == &member_id)
        {
            if member.device(&device_id).is_none() {
                member.add_device(device).map_err(|error| error.to_string())?;
            }
        } else {
            let mut member =
                MemberProfile::new(member_id, invite.display_name.clone()).map_err(|e| e.to_string())?;
            member.add_device(device).map_err(|error| error.to_string())?;
            self.contacts.push(member);
        }

        self.peer_presence.insert(
            invite.device_id.clone(),
            PeerTransportPresence::OfflineButQueueable,
        );

        let chat_id = format!("chat-{}", invite.device_id);
        if !self.chats.iter().any(|chat| chat.id == chat_id) {
            self.chats.push(ChatThreadView {
                id: chat_id.clone(),
                title: invite.display_name.clone(),
                summary: format!("Relay contact on {}", invite.server_name),
                presence_label: "offline but queueable".to_string(),
                presence_state: PresenceStateView::Offline,
                unread_count: 0,
                security_label: "Verification required before elevated trust".to_string(),
                kind: ChatKindView::Direct,
                participants: vec![
                    self.local_profile.display_name().to_string(),
                    invite.display_name.clone(),
                ],
                messages: Vec::new(),
            });
        }
        self.chat_runtime_device_ids
            .insert(chat_id, invite.device_id.clone());
        self.sync_chat_labels();
        Ok(())
    }

    fn remote_display_name_for_device(&self, device_id: &str) -> Option<String> {
        self.contacts.iter().find_map(|member| {
            member
                .devices()
                .find(|device| device.device_id().as_str() == device_id)
                .map(|_| member.display_name().to_string())
        })
    }

    fn drain_connection_events(&mut self) {
        for event in self.connection_manager.drain_events() {
            match event {
                ConnectionEvent::PresenceChanged { device_id, presence } => {
                    self.peer_presence.insert(device_id, presence);
                }
                ConnectionEvent::InboundMessage {
                    device_id,
                    conversation_id,
                    message_id,
                    body,
                    sent_at_unix_ms,
                } => {
                    let author = self
                        .remote_display_name_for_device(&device_id)
                        .unwrap_or_else(|| "Remote peer".to_string());
                    if let Some(chat) = self
                        .chats
                        .iter_mut()
                        .find(|chat| chat.id == conversation_id)
                    {
                        if chat.messages.iter().any(|message| message.id == message_id) {
                            continue;
                        }
                        chat.messages.push(MessageView {
                            id: message_id,
                            author,
                            body: body.clone(),
                            timestamp_label: timestamp_label(sent_at_unix_ms),
                            direction: MessageDirectionView::Inbound,
                            delivery_state: DeliveryStateView::Delivered,
                            forwarded_from: None,
                            reply_preview: None,
                            reactions: Vec::new(),
                            attachments: Vec::new(),
                        });
                        chat.summary = preview(&body);
                        chat.unread_count = chat.unread_count.saturating_add(1);
                    }
                }
            }
        }
    }

    fn peer_views(&self) -> Vec<PeerView> {
        let mut peers = Vec::new();
        let mut seen_device_ids = std::collections::BTreeSet::new();

        for member in &self.contacts {
            for device in member.devices() {
                seen_device_ids.insert(device.device_id().to_string());
                let runtime = self.contact_runtimes.get(device.device_id().as_str());
                let connection_presence = self
                    .peer_presence
                    .get(device.device_id().as_str())
                    .copied()
                    .unwrap_or(PeerTransportPresence::OfflineButQueueable);
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
                        let relay_label = match connection_presence {
                            PeerTransportPresence::LanOnline => "reachable on LAN",
                            PeerTransportPresence::RelayOnline => "reachable through relay",
                            PeerTransportPresence::OfflineButQueueable => "offline but relay queue is available",
                        };
                        (
                            "relay-routed contact".to_string(),
                            None,
                            vec!["messaging-v1".to_string()],
                            match connection_presence {
                                PeerTransportPresence::LanOnline | PeerTransportPresence::RelayOnline => PeerStateCode::Live,
                                PeerTransportPresence::OfflineButQueueable => PeerStateCode::Dormant,
                            },
                            relay_label.to_string(),
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

        for (device_id, peer) in &self.discovered_peers {
            if seen_device_ids.contains(device_id.as_str()) {
                continue;
            }
            peers.push(PeerView {
                member_id: peer.member_id.to_string(),
                device_id: device_id.clone(),
                device_name: peer.device_name.clone(),
                endpoint: peer
                    .socket_address
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| "discovering...".to_string()),
                hostname: peer.hostname.clone(),
                capabilities: peer
                    .capabilities
                    .iter()
                    .map(|c| c.as_str().to_string())
                    .collect(),
                state: PeerStateCode::Dormant,
                trust_label: "new peer".to_string(),
                last_seen_label: "discovered via mDNS".to_string(),
            });
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
        let updates: Vec<(String, String, PresenceStateView, String)> = self
            .chat_runtime_device_ids
            .iter()
            .map(|(chat_id, device_id)| {
                let route_label = match self
                    .peer_presence
                    .get(device_id.as_str())
                    .copied()
                    .unwrap_or(PeerTransportPresence::OfflineButQueueable)
                {
                    PeerTransportPresence::LanOnline => "lan_online".to_string(),
                    PeerTransportPresence::RelayOnline => "relay_online".to_string(),
                    PeerTransportPresence::OfflineButQueueable => "offline_but_queueable".to_string(),
                };
                (
                    chat_id.clone(),
                    self.security_label_for_device(device_id, true),
                    self.presence_state_for_device(device_id),
                    route_label,
                )
            })
            .collect();

        for (chat_id, security_label, presence_state, route_label) in updates {
            if let Some(chat) = self.chats.iter_mut().find(|chat| chat.id == chat_id) {
                chat.presence_label = format!("{} · {}", presence_label(presence_state), route_label);
                chat.presence_state = presence_state;
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

    fn reply_preview_for(
        &self,
        chat_id: &str,
        reply_to_message_id: Option<&str>,
    ) -> Result<Option<String>, String> {
        let Some(message_id) = reply_to_message_id else {
            return Ok(None);
        };
        let chat = self
            .chats
            .iter()
            .find(|chat| chat.id == chat_id)
            .ok_or_else(|| format!("chat '{chat_id}' not found"))?;
        let message = chat
            .messages
            .iter()
            .find(|message| message.id == message_id)
            .ok_or_else(|| format!("message '{message_id}' was not found"))?;
        Ok(Some(preview(&message.body)))
    }

    fn total_unread_count(&self) -> u32 {
        self.chats.iter().map(|chat| chat.unread_count).sum()
    }

    fn push_notification(&mut self, message: String) {
        self.last_notification = message;
        let unread = self.total_unread_count();
        self.tray_status = if unread == 0 {
            "Tray idle".to_string()
        } else {
            format!("{unread} unread")
        };
    }

    fn presence_state_for_device(&self, device_id: &str) -> PresenceStateView {
        let has_runtime = self.contact_runtimes.contains_key(device_id);
        let relay_presence = self
            .peer_presence
            .get(device_id)
            .copied()
            .unwrap_or(PeerTransportPresence::OfflineButQueueable);
        let is_verified = self.contacts.iter().any(|member| {
            member
                .devices()
                .any(|device| device.device_id().as_str() == device_id && device.is_verified())
        });
        match (has_runtime, relay_presence, is_verified) {
            (true, _, true) => PresenceStateView::Online,
            (true, _, false) => PresenceStateView::Reconnecting,
            (false, PeerTransportPresence::LanOnline | PeerTransportPresence::RelayOnline, true) => {
                PresenceStateView::Online
            }
            (false, PeerTransportPresence::LanOnline | PeerTransportPresence::RelayOnline, false) => {
                PresenceStateView::Reconnecting
            }
            (false, PeerTransportPresence::OfflineButQueueable, _) => PresenceStateView::Offline,
        }
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

fn attachment_body(file_name: &str, mime_type: &str) -> String {
    if mime_type.starts_with("image/") {
        format!("Shared photo: {file_name}")
    } else if mime_type == "application/pdf" {
        format!("Shared document: {file_name}")
    } else {
        format!("Shared file: {file_name}")
    }
}

fn sample_photo_attachment() -> MessageAttachmentView {
    MessageAttachmentView {
        id: "sample-photo-1".to_string(),
        file_name: "verification-board.jpg".to_string(),
        mime_type: "image/svg+xml".to_string(),
        size_label: "18 KB".to_string(),
        transfer_route: "server_blob_store".to_string(),
        status_label: "encrypted relay blob cached".to_string(),
        preview_data_url: Some(sample_photo_preview_data_url()),
        blob_id: Some("blob-sample-photo".to_string()),
        upload_progress: 1.0,
    }
}

fn sample_voice_attachment() -> MessageAttachmentView {
    MessageAttachmentView {
        id: "sample-voice-1".to_string(),
        file_name: "voice-note.wav".to_string(),
        mime_type: "audio/wav".to_string(),
        size_label: "12 KB".to_string(),
        transfer_route: "server_blob_store".to_string(),
        status_label: "voice note synced".to_string(),
        preview_data_url: Some(sample_voice_data_url()),
        blob_id: Some("blob-sample-voice".to_string()),
        upload_progress: 1.0,
    }
}

fn sample_pdf_attachment() -> MessageAttachmentView {
    MessageAttachmentView {
        id: "sample-pdf-1".to_string(),
        file_name: "relay-hardening-brief.pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        size_label: "14 KB".to_string(),
        transfer_route: "server_blob_store".to_string(),
        status_label: "document preview ready".to_string(),
        preview_data_url: Some(sample_pdf_data_url()),
        blob_id: Some("blob-sample-pdf".to_string()),
        upload_progress: 1.0,
    }
}

fn sample_photo_preview_data_url() -> String {
    data_url(
        "image/svg+xml",
        br#"<svg xmlns='http://www.w3.org/2000/svg' width='480' height='320' viewBox='0 0 480 320'>
<defs>
<linearGradient id='g' x1='0%' y1='0%' x2='100%' y2='100%'>
<stop offset='0%' stop-color='#16354a'/>
<stop offset='100%' stop-color='#f07c3e'/>
</linearGradient>
</defs>
<rect width='480' height='320' rx='26' fill='url(#g)'/>
<circle cx='104' cy='92' r='34' fill='#ffe8a8' opacity='0.85'/>
<path d='M54 246 152 142l70 70 48-40 102 74H54Z' fill='#0e2231' opacity='0.92'/>
<path d='M162 246 248 156l54 54 34-24 88 60H162Z' fill='#e9f2f6' opacity='0.76'/>
<text x='34' y='42' fill='#f7f4ec' font-size='22' font-family='IBM Plex Sans, sans-serif'>Relay photo preview</text>
</svg>"#,
    )
}

fn sample_voice_data_url() -> String {
    data_url(
        "audio/wav",
        &[
            82, 73, 70, 70, 44, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0, 1, 0, 1,
            0, 64, 31, 0, 0, 128, 62, 0, 0, 2, 0, 16, 0, 100, 97, 116, 97, 8, 0, 0, 0, 0, 0, 20,
            10, 20, 10, 0, 0,
        ],
    )
}

fn sample_pdf_data_url() -> String {
    data_url(
        "application/pdf",
        br#"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 180] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length 58 >>
stream
BT /F1 18 Tf 32 110 Td (Relay hardening brief preview) Tj ET
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000241 00000 n
0000000349 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
419
%%EOF"#,
    )
}

fn byte_count_label(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes_f64 = bytes as f64;
    if bytes_f64 >= MB {
        format!("{:.1} MB", bytes_f64 / MB)
    } else if bytes_f64 >= KB {
        format!("{:.0} KB", bytes_f64 / KB)
    } else {
        format!("{bytes} B")
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

fn invite_preview_view(preview: &InvitePreview) -> InvitePreviewView {
    InvitePreviewView {
        invite_id: preview.invite_id.clone(),
        label: preview.label.clone(),
        server_addr: preview.server_addr.clone(),
        server_name: preview.server_name.clone(),
        expires_at_label: timestamp_label(preview.expires_at_unix_ms),
        max_uses: preview.max_uses,
    }
}

fn contact_invite_preview_view(preview: &ContactInvitePreview) -> ContactInvitePreviewView {
    ContactInvitePreviewView {
        member_id: preview.member_id.clone(),
        device_id: preview.device_id.clone(),
        display_name: preview.display_name.clone(),
        server_addr: preview.server_addr.clone(),
        server_name: preview.server_name.clone(),
        expires_at_label: timestamp_label(preview.expires_at_unix_ms),
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

// ── Persistent-identity helpers ───────────────────────────────────────────────

/// Load a 32-byte bootstrap storage key from `path`, or generate a fresh one
/// and persist it so every subsequent launch uses the same key.
fn load_or_create_storage_key(
    path: &std::path::Path,
    rng: &mut impl rand_core::RngCore,
) -> Result<StorageKey, String> {
    if path.exists() {
        let raw = fs::read(path).map_err(|e| format!("read bootstrap.key: {e}"))?;
        let arr: [u8; 32] = raw
            .try_into()
            .map_err(|_| "bootstrap.key has wrong length — delete it to regenerate".to_string())?;
        Ok(StorageKey::from_bytes(arr))
    } else {
        let mut raw = [0u8; 32];
        rng.fill_bytes(&mut raw);
        fs::write(path, &raw).map_err(|e| format!("write bootstrap.key: {e}"))?;
        Ok(StorageKey::from_bytes(raw))
    }
}

/// Return a human-readable display name for this user.
///
/// Priority:
///   1. `LOCALMESSENGER_DISPLAY_NAME` environment variable
///   2. `USERNAME` (Windows) / `USER` (Unix)
///   3. `LOGNAME` (POSIX fallback)
///   4. `"User"` hard-coded fallback
fn resolve_display_name() -> String {
    std::env::var("LOCALMESSENGER_DISPLAY_NAME")
        .or_else(|_| std::env::var("USERNAME"))
        .or_else(|_| std::env::var("USER"))
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "User".to_string())
}

/// Return a short device-name suffix (OS hostname when available).
fn system_device_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Desktop".to_string())
}

fn now_unix_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn presence_label(presence_state: PresenceStateView) -> &'static str {
    match presence_state {
        PresenceStateView::Online => "secure session active · online",
        PresenceStateView::Reconnecting => "runtime reachable, verification pending",
        PresenceStateView::Offline => "offline",
    }
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
    use base64::Engine;
    use localmessenger_server_protocol::{
        InviteClaims, SERVER_PROTOCOL_VERSION, encode_invite_link,
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
            .send_message("chat-bob", "UI smoke message", None)
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

    #[tokio::test]
    async fn invite_preview_populates_onboarding_state() {
        let mut state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");
        let invite_link = encode_invite_link(
            b"secret",
            &InviteClaims {
                version: SERVER_PROTOCOL_VERSION,
                invite_id: "inv-preview".to_string(),
                label: "Kitchen relay".to_string(),
                server_addr: "127.0.0.1:7443".to_string(),
                server_name: "relay.local".to_string(),
                server_certificate_der_base64: base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode([1_u8; 4]),
                issued_at_unix_ms: 10,
                expires_at_unix_ms: 20,
                max_uses: 3,
            },
        )
        .expect("invite link");

        state
            .preview_invite(&invite_link)
            .expect("preview should succeed");
        let snapshot = state.snapshot();
        assert_eq!(
            snapshot
                .onboarding
                .invite_preview
                .as_ref()
                .expect("preview")
                .server_name,
            "relay.local"
        );
    }

    #[tokio::test]
    async fn bootstrap_snapshot_contains_photo_attachment_preview() {
        let state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");
        let snapshot = state.snapshot();
        let attachment = snapshot
            .chats
            .iter()
            .find(|chat| chat.id == "chat-carol")
            .and_then(|chat| chat.messages.first())
            .and_then(|message| message.attachments.first())
            .expect("photo attachment should exist");

        assert_eq!(attachment.transfer_route, "server_blob_store");
        assert!(
            attachment
                .preview_data_url
                .as_ref()
                .expect("preview")
                .starts_with("data:image/")
        );
    }

    #[tokio::test]
    async fn bootstrap_snapshot_contains_voice_attachment_and_notifications() {
        let state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");
        let snapshot = state.snapshot();
        let voice_attachment = snapshot
            .chats
            .iter()
            .find(|chat| chat.id == "chat-bob")
            .and_then(|chat| chat.messages.get(1))
            .and_then(|message| message.attachments.first())
            .expect("voice attachment should exist");

        assert_eq!(voice_attachment.mime_type, "audio/wav");
        assert!(
            voice_attachment
                .preview_data_url
                .as_ref()
                .expect("audio preview")
                .starts_with("data:audio/")
        );
        assert_eq!(snapshot.notifications.tray_label, "Tray idle");
    }

    #[tokio::test]
    async fn send_message_can_include_reply_preview() {
        let mut state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");

        state
            .send_message("chat-bob", "Reply path works", Some("m-1"))
            .await
            .expect("reply should send");

        let snapshot = state.snapshot();
        let message = snapshot
            .chats
            .iter()
            .find(|chat| chat.id == "chat-bob")
            .and_then(|chat| {
                chat.messages
                    .iter()
                    .rev()
                    .find(|entry| entry.author == "Rimus")
            })
            .expect("message should exist");
        assert_eq!(
            message.reply_preview.as_deref(),
            Some("I am back on the local runtime. QUIC path is stable now.")
        );
    }

    #[tokio::test]
    async fn forward_and_react_update_snapshot() {
        let mut state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");

        state
            .forward_message("chat-bob", "chat-carol", "m-2")
            .expect("forward should succeed");
        state
            .toggle_reaction("chat-carol", "c-1", "👍")
            .expect("reaction should succeed");

        let snapshot = state.snapshot();
        let forwarded = snapshot
            .chats
            .iter()
            .find(|chat| chat.id == "chat-carol")
            .and_then(|chat| chat.messages.last())
            .expect("forwarded message");
        assert_eq!(forwarded.forwarded_from.as_deref(), Some("Bob"));

        let reacted = snapshot
            .chats
            .iter()
            .find(|chat| chat.id == "chat-carol")
            .and_then(|chat| chat.messages.iter().find(|message| message.id == "c-1"))
            .expect("reacted message");
        assert!(reacted.reactions.iter().any(|reaction| reaction == "👍"));
    }

    #[tokio::test]
    async fn group_send_message_fan_out_delivers_and_receives_replies() {
        let mut state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");

        // Verify the group chat is live (not staged) after bootstrap.
        let snapshot = state.snapshot();
        let group_chat = snapshot
            .chats
            .iter()
            .find(|chat| chat.id == "chat-lan-crew")
            .expect("LAN Crew chat should exist");
        assert!(
            matches!(group_chat.presence_state, super::PresenceStateView::Online),
            "group must be online after bootstrap"
        );
        assert!(
            group_chat.security_label.contains("epoch 0"),
            "security label should contain epoch"
        );

        // Send a group message — this must NOT return the old stub error.
        state
            .send_message("chat-lan-crew", "Hello group!", None)
            .await
            .expect("group send should succeed");

        let snapshot = state.snapshot();
        let chat = snapshot
            .chats
            .iter()
            .find(|chat| chat.id == "chat-lan-crew")
            .expect("LAN Crew chat should exist after send");

        // Our outbound message must be visible.
        let outbound = chat
            .messages
            .iter()
            .find(|msg| {
                matches!(msg.direction, super::MessageDirectionView::Outbound)
                    && msg.body == "Hello group!"
            })
            .expect("outbound group message should appear in chat");
        assert!(
            matches!(outbound.delivery_state, super::DeliveryStateView::Delivered),
            "delivery state should be Delivered"
        );

        // At least one reply from a member with a reply script should appear.
        let inbound_count = chat
            .messages
            .iter()
            .filter(|msg| matches!(msg.direction, super::MessageDirectionView::Inbound))
            .count();
        assert!(
            inbound_count >= 1,
            "at least one group member should have replied; got {inbound_count}"
        );

        // Presence label must reflect how many members were reached.
        assert!(
            chat.presence_label.contains("members reached"),
            "presence label should report members reached: {}",
            chat.presence_label
        );
    }

    #[tokio::test]
    async fn bootstrap_snapshot_contains_pdf_attachment_and_updater_status() {
        let state = ClientState::bootstrap()
            .await
            .expect("client state should bootstrap");
        let snapshot = state.snapshot();
        let pdf_attachment = snapshot
            .chats
            .iter()
            .find(|chat| chat.id == "chat-carol")
            .and_then(|chat| chat.messages.first())
            .and_then(|message| {
                message
                    .attachments
                    .iter()
                    .find(|attachment| attachment.mime_type == "application/pdf")
            })
            .expect("pdf attachment should exist");

        assert!(
            pdf_attachment
                .preview_data_url
                .as_ref()
                .expect("pdf preview")
                .starts_with("data:application/pdf")
        );
        assert_eq!(snapshot.updater.current_version, "1.0.0");
    }
}
