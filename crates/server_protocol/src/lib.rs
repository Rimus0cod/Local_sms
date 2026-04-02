#![forbid(unsafe_code)]

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use localmessenger_core::{DeviceId, MemberId};
use localmessenger_crypto::{IdentityKeyPair, IdentityPublicKeys, PublicPrekeyBundle};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub const SERVER_PROTOCOL_VERSION: u8 = 1;
pub const AUTH_CONTEXT: &[u8] = b"localmessenger/server-auth/v1";
pub const INVITE_LINK_PREFIX: &str = "localmessenger://join?token=";
pub const CONTACT_INVITE_CONTEXT: &[u8] = b"localmessenger/contact-invite/v1";
pub const CONTACT_INVITE_LINK_PREFIX: &str = "localmessenger://contact?token=";
pub const MAX_RELAY_BLOB_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_BLOB_CHUNK_BYTES: usize = 64 * 1024;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRegistrationBundle {
    pub member_id: String,
    pub device_id: String,
    pub device_name: String,
    pub auth_public_key: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteClaims {
    pub version: u8,
    pub invite_id: String,
    pub label: String,
    pub server_addr: String,
    pub server_name: String,
    pub server_certificate_der_base64: String,
    pub issued_at_unix_ms: i64,
    pub expires_at_unix_ms: i64,
    pub max_uses: u32,
}

impl InviteClaims {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != SERVER_PROTOCOL_VERSION {
            return Err(format!("unsupported invite version {}", self.version));
        }
        if self.invite_id.trim().is_empty() {
            return Err("invite id cannot be empty".to_string());
        }
        if self.label.trim().is_empty() {
            return Err("invite label cannot be empty".to_string());
        }
        if self.server_addr.trim().is_empty() {
            return Err("server addr cannot be empty".to_string());
        }
        if self.server_name.trim().is_empty() {
            return Err("server name cannot be empty".to_string());
        }
        if self.expires_at_unix_ms <= self.issued_at_unix_ms {
            return Err("invite expiry must be after issue time".to_string());
        }
        if self.max_uses == 0 {
            return Err("invite must allow at least one use".to_string());
        }
        decode_invite_certificate(self)?;
        Ok(())
    }
}

impl DeviceContactInvite {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != SERVER_PROTOCOL_VERSION {
            return Err(format!(
                "unsupported contact invite version {}",
                self.version
            ));
        }
        MemberId::new(self.member_id.clone()).map_err(|error| error.to_string())?;
        DeviceId::new(self.device_id.clone()).map_err(|error| error.to_string())?;
        if self.display_name.trim().is_empty() {
            return Err("contact display name cannot be empty".to_string());
        }
        if self.server_addr.trim().is_empty() {
            return Err("contact server addr cannot be empty".to_string());
        }
        if self.server_name.trim().is_empty() {
            return Err("contact server name cannot be empty".to_string());
        }
        if self.expires_at_unix_ms <= self.issued_at_unix_ms {
            return Err("contact invite expiry must be after issue time".to_string());
        }
        decode_contact_invite_server_certificate(self)?;
        decode_contact_invite_device_transport_certificate(self)?;
        self.prekey_bundle
            .verify()
            .map_err(|error| error.to_string())?;
        if self.prekey_bundle.identity != self.identity_keys {
            return Err("contact invite prekey bundle identity mismatch".to_string());
        }
        verify_contact_invite(self)?;
        Ok(())
    }

    pub fn unsigned_payload(&self) -> Result<Vec<u8>, String> {
        let unsigned = UnsignedDeviceContactInvite {
            version: self.version,
            member_id: self.member_id.clone(),
            device_id: self.device_id.clone(),
            display_name: self.display_name.clone(),
            server_addr: self.server_addr.clone(),
            server_name: self.server_name.clone(),
            server_certificate_der_base64: self.server_certificate_der_base64.clone(),
            device_transport_certificate_der_base64: self
                .device_transport_certificate_der_base64
                .clone(),
            identity_keys: self.identity_keys.clone(),
            prekey_bundle: self.prekey_bundle.clone(),
            issued_at_unix_ms: self.issued_at_unix_ms,
            expires_at_unix_ms: self.expires_at_unix_ms,
        };
        serde_json::to_vec(&unsigned).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct UnsignedDeviceContactInvite {
    version: u8,
    member_id: String,
    device_id: String,
    display_name: String,
    server_addr: String,
    server_name: String,
    server_certificate_der_base64: String,
    device_transport_certificate_der_base64: String,
    identity_keys: IdentityPublicKeys,
    prekey_bundle: PublicPrekeyBundle,
    issued_at_unix_ms: i64,
    expires_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitePreview {
    pub invite_id: String,
    pub label: String,
    pub server_addr: String,
    pub server_name: String,
    pub expires_at_unix_ms: i64,
    pub max_uses: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceContactInvite {
    pub version: u8,
    pub member_id: String,
    pub device_id: String,
    pub display_name: String,
    pub server_addr: String,
    pub server_name: String,
    pub server_certificate_der_base64: String,
    pub device_transport_certificate_der_base64: String,
    pub identity_keys: IdentityPublicKeys,
    pub prekey_bundle: PublicPrekeyBundle,
    pub issued_at_unix_ms: i64,
    pub expires_at_unix_ms: i64,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactInvitePreview {
    pub member_id: String,
    pub device_id: String,
    pub display_name: String,
    pub server_addr: String,
    pub server_name: String,
    pub expires_at_unix_ms: i64,
}

impl DeviceRegistrationBundle {
    pub fn new(
        member_id: &MemberId,
        device_id: &DeviceId,
        device_name: impl Into<String>,
        auth_public_key: [u8; 32],
    ) -> Self {
        Self {
            member_id: member_id.to_string(),
            device_id: device_id.to_string(),
            device_name: device_name.into(),
            auth_public_key,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        MemberId::new(self.member_id.clone()).map_err(|error| error.to_string())?;
        DeviceId::new(self.device_id.clone()).map_err(|error| error.to_string())?;

        if self.device_name.trim().is_empty() {
            return Err("device name cannot be empty".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHello {
    pub version: u8,
    pub member_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub version: u8,
    pub nonce: [u8; 32],
    pub issued_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthResponse {
    pub version: u8,
    pub member_id: String,
    pub device_id: String,
    pub nonce: [u8; 32],
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthOk {
    pub version: u8,
    pub member_id: String,
    pub device_id: String,
    pub device_name: String,
    pub server_time_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinWithInvite {
    pub invite_link: String,
    pub registration: DeviceRegistrationBundle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinAccepted {
    pub version: u8,
    pub invite_id: String,
    pub member_id: String,
    pub device_id: String,
    pub server_addr: String,
    pub server_name: String,
    pub server_certificate_der_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerRelayFrame {
    pub request_id: u64,
    pub recipient_device_id: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerFrame {
    pub sender_device_id: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerQueued {
    pub request_id: u64,
    pub recipient_device_id: String,
    pub queued_at_unix_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Photo,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobUploadStart {
    pub request_id: u64,
    pub file_name: String,
    pub mime_type: String,
    pub media_kind: MediaKind,
    pub plaintext_bytes: u64,
    pub ciphertext_bytes: u64,
    pub sha256_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobUploadReady {
    pub request_id: u64,
    pub blob_id: String,
    pub max_chunk_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobUploadChunk {
    pub request_id: u64,
    pub blob_id: String,
    pub offset: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobChunkAck {
    pub request_id: u64,
    pub blob_id: String,
    pub received_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobUploadFinish {
    pub request_id: u64,
    pub blob_id: String,
    pub ciphertext_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredBlob {
    pub blob_id: String,
    pub uploaded_by_device_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub media_kind: MediaKind,
    pub plaintext_bytes: u64,
    pub ciphertext_bytes: u64,
    pub sha256_hex: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobStored {
    pub request_id: u64,
    pub blob: StoredBlob,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobDownloadRequest {
    pub request_id: u64,
    pub blob_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobDownloadReady {
    pub request_id: u64,
    pub blob: StoredBlob,
    pub max_chunk_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobDownloadChunk {
    pub request_id: u64,
    pub blob_id: String,
    pub offset: u64,
    pub bytes: Vec<u8>,
    pub is_last: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRejected {
    pub request_id: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerUnavailableReason {
    Offline,
    UnknownRecipient,
    Disabled,
    Unauthorized,
    RateLimited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerUnavailable {
    pub request_id: u64,
    pub recipient_device_id: String,
    pub reason: PeerUnavailableReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disconnect {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub version: u8,
    pub server_time_unix_ms: i64,
    pub online_devices: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientEnvelope {
    JoinWithInvite(JoinWithInvite),
    AuthHello(AuthHello),
    AuthResponse(AuthResponse),
    PeerRelayFrame(PeerRelayFrame),
    BlobUploadStart(BlobUploadStart),
    BlobUploadChunk(BlobUploadChunk),
    BlobUploadFinish(BlobUploadFinish),
    BlobDownloadRequest(BlobDownloadRequest),
    HealthCheck,
    Disconnect(Disconnect),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerEnvelope {
    JoinAccepted(JoinAccepted),
    AuthChallenge(AuthChallenge),
    AuthOk(AuthOk),
    PeerFrame(PeerFrame),
    PeerQueued(PeerQueued),
    PeerUnavailable(PeerUnavailable),
    BlobUploadReady(BlobUploadReady),
    BlobChunkAck(BlobChunkAck),
    BlobStored(BlobStored),
    BlobDownloadReady(BlobDownloadReady),
    BlobDownloadChunk(BlobDownloadChunk),
    BlobRejected(BlobRejected),
    Health(Health),
    Disconnect(Disconnect),
}

impl BlobUploadStart {
    pub fn validate(&self) -> Result<(), String> {
        if self.file_name.trim().is_empty() {
            return Err("file name cannot be empty".to_string());
        }
        if self.mime_type.trim().is_empty() {
            return Err("mime type cannot be empty".to_string());
        }
        if self.ciphertext_bytes == 0 {
            return Err("ciphertext bytes must be greater than zero".to_string());
        }
        if self.ciphertext_bytes > MAX_RELAY_BLOB_BYTES {
            return Err(format!(
                "ciphertext exceeds relay blob limit of {} bytes",
                MAX_RELAY_BLOB_BYTES
            ));
        }
        if self.sha256_hex.len() != 64
            || !self
                .sha256_hex
                .chars()
                .all(|value| value.is_ascii_hexdigit())
        {
            return Err("sha256 hex digest must be 64 hex characters".to_string());
        }
        Ok(())
    }
}

pub fn auth_challenge_payload(member_id: &str, device_id: &str, nonce: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(
        AUTH_CONTEXT.len() + member_id.len() + device_id.len() + nonce.len() + 2,
    );
    payload.extend_from_slice(AUTH_CONTEXT);
    payload.push(0);
    payload.extend_from_slice(member_id.as_bytes());
    payload.push(0xff);
    payload.extend_from_slice(device_id.as_bytes());
    payload.extend_from_slice(nonce);
    payload
}

pub fn sign_invite_token(secret: &[u8], claims: &InviteClaims) -> Result<String, String> {
    claims.validate()?;
    let payload = serde_json::to_vec(claims).map_err(|error| error.to_string())?;
    let payload_segment = URL_SAFE_NO_PAD.encode(payload);
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|error| format!("invalid HMAC secret: {error}"))?;
    mac.update(payload_segment.as_bytes());
    let signature_segment = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("{payload_segment}.{signature_segment}"))
}

pub fn verify_invite_token(secret: &[u8], token: &str) -> Result<InviteClaims, String> {
    let (payload_segment, signature_segment) = token
        .split_once('.')
        .ok_or_else(|| "invite token must contain payload and signature".to_string())?;
    let signature = URL_SAFE_NO_PAD
        .decode(signature_segment)
        .map_err(|error| error.to_string())?;
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|error| format!("invalid HMAC secret: {error}"))?;
    mac.update(payload_segment.as_bytes());
    mac.verify_slice(&signature)
        .map_err(|_| "invite signature verification failed".to_string())?;

    let payload = URL_SAFE_NO_PAD
        .decode(payload_segment)
        .map_err(|error| error.to_string())?;
    let claims: InviteClaims =
        serde_json::from_slice(&payload).map_err(|error| error.to_string())?;
    claims.validate()?;
    Ok(claims)
}

pub fn encode_invite_link(secret: &[u8], claims: &InviteClaims) -> Result<String, String> {
    let token = sign_invite_token(secret, claims)?;
    Ok(format!("{INVITE_LINK_PREFIX}{token}"))
}

pub fn parse_invite_link(link: &str) -> Result<&str, String> {
    link.strip_prefix(INVITE_LINK_PREFIX)
        .ok_or_else(|| "invite link must start with localmessenger://join?token=".to_string())
}

pub fn verify_invite_link(secret: &[u8], link: &str) -> Result<InviteClaims, String> {
    verify_invite_token(secret, parse_invite_link(link)?)
}

pub fn invite_preview_from_claims(claims: &InviteClaims) -> InvitePreview {
    InvitePreview {
        invite_id: claims.invite_id.clone(),
        label: claims.label.clone(),
        server_addr: claims.server_addr.clone(),
        server_name: claims.server_name.clone(),
        expires_at_unix_ms: claims.expires_at_unix_ms,
        max_uses: claims.max_uses,
    }
}

pub fn decode_invite_certificate(claims: &InviteClaims) -> Result<Vec<u8>, String> {
    URL_SAFE_NO_PAD
        .decode(&claims.server_certificate_der_base64)
        .map_err(|error| error.to_string())
}

pub fn sign_contact_invite(
    identity: &IdentityKeyPair,
    invite: &mut DeviceContactInvite,
) -> Result<(), String> {
    let payload = contact_invite_signature_payload(invite)?;
    invite.signature = identity.sign_message(&payload).to_vec();
    Ok(())
}

pub fn verify_contact_invite(invite: &DeviceContactInvite) -> Result<(), String> {
    let payload = contact_invite_signature_payload(invite)?;
    invite
        .identity_keys
        .verify_message(&payload, &invite.signature)
        .map_err(|error| error.to_string())
}

pub fn encode_contact_invite_link(invite: &DeviceContactInvite) -> Result<String, String> {
    invite.validate()?;
    let payload = serde_json::to_vec(invite).map_err(|error| error.to_string())?;
    Ok(format!(
        "{CONTACT_INVITE_LINK_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(payload)
    ))
}

pub fn parse_contact_invite_link(link: &str) -> Result<DeviceContactInvite, String> {
    let token = link
        .strip_prefix(CONTACT_INVITE_LINK_PREFIX)
        .ok_or_else(|| {
            "contact invite link must start with localmessenger://contact?token=".to_string()
        })?;
    let payload = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|error| error.to_string())?;
    let invite: DeviceContactInvite =
        serde_json::from_slice(&payload).map_err(|error| error.to_string())?;
    invite.validate()?;
    Ok(invite)
}

pub fn contact_invite_preview(invite: &DeviceContactInvite) -> ContactInvitePreview {
    ContactInvitePreview {
        member_id: invite.member_id.clone(),
        device_id: invite.device_id.clone(),
        display_name: invite.display_name.clone(),
        server_addr: invite.server_addr.clone(),
        server_name: invite.server_name.clone(),
        expires_at_unix_ms: invite.expires_at_unix_ms,
    }
}

pub fn decode_contact_invite_server_certificate(
    invite: &DeviceContactInvite,
) -> Result<Vec<u8>, String> {
    URL_SAFE_NO_PAD
        .decode(&invite.server_certificate_der_base64)
        .map_err(|error| error.to_string())
}

pub fn decode_contact_invite_device_transport_certificate(
    invite: &DeviceContactInvite,
) -> Result<Vec<u8>, String> {
    URL_SAFE_NO_PAD
        .decode(&invite.device_transport_certificate_der_base64)
        .map_err(|error| error.to_string())
}

fn contact_invite_signature_payload(invite: &DeviceContactInvite) -> Result<Vec<u8>, String> {
    let unsigned = invite.unsigned_payload()?;
    let mut payload = Vec::with_capacity(CONTACT_INVITE_CONTEXT.len() + unsigned.len());
    payload.extend_from_slice(CONTACT_INVITE_CONTEXT);
    payload.extend_from_slice(&unsigned);
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::{
        BlobUploadStart, CONTACT_INVITE_LINK_PREFIX, DeviceContactInvite, DeviceRegistrationBundle,
        INVITE_LINK_PREFIX, InviteClaims, MAX_RELAY_BLOB_BYTES, MediaKind,
        auth_challenge_payload, contact_invite_preview, encode_contact_invite_link,
        encode_invite_link, invite_preview_from_claims, parse_contact_invite_link,
        sign_contact_invite, verify_invite_link,
    };
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use localmessenger_core::{DeviceId, MemberId};
    use localmessenger_crypto::{IdentityKeyPair, LocalPrekeyStore};
    use rand_core::OsRng;

    #[test]
    fn registration_bundle_validates_ids() {
        let bundle = DeviceRegistrationBundle::new(
            &MemberId::new("alice").expect("member id"),
            &DeviceId::new("alice-phone").expect("device id"),
            "Alice Phone",
            [7_u8; 32],
        );

        assert!(bundle.validate().is_ok());
        assert!(!auth_challenge_payload("alice", "alice-phone", &[1_u8; 32]).is_empty());
    }

    #[test]
    fn signed_invite_link_round_trip_verifies() {
        let claims = InviteClaims {
            version: super::SERVER_PROTOCOL_VERSION,
            invite_id: "inv-1".to_string(),
            label: "Home relay".to_string(),
            server_addr: "203.0.113.10:7443".to_string(),
            server_name: "relay.local".to_string(),
            server_certificate_der_base64: URL_SAFE_NO_PAD.encode([7_u8; 8]),
            issued_at_unix_ms: 100,
            expires_at_unix_ms: 200,
            max_uses: 3,
        };

        let link = encode_invite_link(b"secret", &claims).expect("link");
        assert!(link.starts_with(INVITE_LINK_PREFIX));

        let verified = verify_invite_link(b"secret", &link).expect("verify");
        assert_eq!(verified.invite_id, "inv-1");
        assert_eq!(invite_preview_from_claims(&verified).label, "Home relay");
        assert!(verify_invite_link(b"wrong", &link).is_err());
    }

    #[test]
    fn signed_contact_invite_round_trip_verifies() {
        let mut rng = OsRng;
        let identity = IdentityKeyPair::generate(&mut rng);
        let prekeys = LocalPrekeyStore::generate(&mut rng, &identity, 7, 0, 0);
        let mut invite = DeviceContactInvite {
            version: super::SERVER_PROTOCOL_VERSION,
            member_id: "alice".to_string(),
            device_id: "alice-phone".to_string(),
            display_name: "Alice".to_string(),
            server_addr: "203.0.113.10:7443".to_string(),
            server_name: "relay.local".to_string(),
            server_certificate_der_base64: URL_SAFE_NO_PAD.encode([1_u8; 8]),
            device_transport_certificate_der_base64: URL_SAFE_NO_PAD.encode([2_u8; 8]),
            identity_keys: identity.public_keys(),
            prekey_bundle: prekeys.public_bundle(),
            issued_at_unix_ms: 100,
            expires_at_unix_ms: 200,
            signature: Vec::new(),
        };
        sign_contact_invite(&identity, &mut invite).expect("sign");

        let link = encode_contact_invite_link(&invite).expect("link");
        assert!(link.starts_with(CONTACT_INVITE_LINK_PREFIX));

        let parsed = parse_contact_invite_link(&link).expect("parse");
        let preview = contact_invite_preview(&parsed);
        assert_eq!(preview.device_id, "alice-phone");
        assert_eq!(parsed.signature, invite.signature);
    }

    #[test]
    fn blob_upload_start_enforces_limits() {
        let valid = BlobUploadStart {
            request_id: 7,
            file_name: "photo.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            media_kind: MediaKind::Photo,
            plaintext_bytes: 1024,
            ciphertext_bytes: 2048,
            sha256_hex: "a".repeat(64),
        };
        assert!(valid.validate().is_ok());

        let mut too_large = valid.clone();
        too_large.ciphertext_bytes = MAX_RELAY_BLOB_BYTES + 1;
        assert!(too_large.validate().is_err());
    }
}
