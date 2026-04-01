use std::collections::HashMap;
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use base64::Engine;
use localmessenger_core::{Device, DeviceId};
use localmessenger_crypto::{IdentityKeyMaterial, IdentityKeyPair};
use localmessenger_messaging::FrameChannel;
use localmessenger_server_protocol::{
    AuthHello, AuthResponse, BlobChunkAck, BlobDownloadChunk, BlobDownloadReady,
    BlobDownloadRequest, BlobRejected, BlobStored, BlobUploadChunk, BlobUploadFinish,
    BlobUploadReady, BlobUploadStart, ClientEnvelope, DeviceRegistrationBundle, InvitePreview,
    JoinAccepted, JoinWithInvite, MAX_BLOB_CHUNK_BYTES, PeerUnavailableReason,
    SERVER_PROTOCOL_VERSION, ServerEnvelope, StoredBlob, auth_challenge_payload,
    decode_invite_certificate, invite_preview_from_claims,
};
use localmessenger_transport::{
    ReconnectPolicy, TransportConnection, TransportEndpoint, TransportEndpointConfig,
    TransportError, TransportFrame, TransportIdentity,
};
use tokio::sync::{Mutex, mpsc, oneshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportRoute {
    ServerRelay,
    DirectLan,
}

impl TransportRoute {
    pub fn label(self) -> &'static str {
        match self {
            Self::ServerRelay => "server_relay",
            Self::DirectLan => "direct_lan",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RelayClientConfig {
    pub server_addr: SocketAddr,
    pub server_name: String,
    pub trusted_server_certificate_der: Vec<u8>,
    pub auth_device_id: String,
    pub preferred_routes: Vec<TransportRoute>,
}

impl RelayClientConfig {
    pub fn from_env(default_device_id: &str) -> Result<Option<Self>, String> {
        let Some(server_addr) = std::env::var("LOCALMESSENGER_SERVER_ADDR").ok() else {
            return Ok(None);
        };
        let cert_path = std::env::var("LOCALMESSENGER_SERVER_CERT_DER").map_err(|_| {
            "LOCALMESSENGER_SERVER_CERT_DER must point to a DER certificate".to_string()
        })?;
        let trusted_server_certificate_der =
            std::fs::read(&cert_path).map_err(|error| error.to_string())?;
        let server_name = std::env::var("LOCALMESSENGER_SERVER_NAME")
            .unwrap_or_else(|_| "relay.local".to_string());
        let auth_device_id = std::env::var("LOCALMESSENGER_SERVER_DEVICE_ID")
            .unwrap_or_else(|_| default_device_id.to_string());
        let preferred_routes = parse_transport_order(
            std::env::var("LOCALMESSENGER_TRANSPORT_ORDER")
                .unwrap_or_else(|_| "server_relay,direct_lan".to_string()),
        );

        Ok(Some(Self {
            server_addr: server_addr
                .parse()
                .map_err(|error| format!("invalid LOCALMESSENGER_SERVER_ADDR: {error}"))?,
            server_name,
            trusted_server_certificate_der,
            auth_device_id,
            preferred_routes,
        }))
    }

    pub fn from_join_accepted(
        accepted: &JoinAccepted,
        auth_device_id: impl Into<String>,
        preferred_routes: Vec<TransportRoute>,
    ) -> Result<Self, String> {
        let trusted_server_certificate_der =
            decode_join_accepted_certificate(accepted.server_certificate_der_base64.as_str())?;
        Ok(Self {
            server_addr: accepted
                .server_addr
                .parse()
                .map_err(|error| format!("invalid invite server address: {error}"))?,
            server_name: accepted.server_name.clone(),
            trusted_server_certificate_der,
            auth_device_id: auth_device_id.into(),
            preferred_routes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayServerStatus {
    Disabled,
    Connected,
    Failed,
}

impl RelayServerStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Connected => "connected",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayAuthStatus {
    Disabled,
    Authenticated,
    Failed,
}

impl RelayAuthStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Authenticated => "authenticated",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone)]
pub struct RelayBootstrap {
    pub client: RelayClient,
    pub server_status: RelayServerStatus,
    pub auth_status: RelayAuthStatus,
}

struct RelayShared {
    pending_route_results: Mutex<HashMap<u64, oneshot::Sender<RouteSendResult>>>,
    pending_blob_starts: Mutex<HashMap<u64, oneshot::Sender<Result<BlobUploadReady, String>>>>,
    pending_blob_chunks: Mutex<HashMap<u64, oneshot::Sender<Result<BlobChunkAck, String>>>>,
    pending_blob_finishes: Mutex<HashMap<u64, oneshot::Sender<Result<BlobStored, String>>>>,
    pending_blob_downloads: Mutex<HashMap<u64, mpsc::UnboundedSender<BlobDownloadEvent>>>,
    inboxes: Mutex<HashMap<String, mpsc::UnboundedSender<TransportFrame>>>,
}

#[derive(Debug)]
enum RouteSendResult {
    Queued,
    Unavailable(PeerUnavailableReason),
}

#[derive(Debug)]
enum BlobDownloadEvent {
    Ready(BlobDownloadReady),
    Chunk(BlobDownloadChunk),
    Rejected(String),
}

#[derive(Debug, Clone)]
pub struct UploadedBlobHandle {
    pub metadata: StoredBlob,
}

#[derive(Debug, Clone)]
pub struct DownloadedBlob {
    pub metadata: StoredBlob,
    pub ciphertext: Vec<u8>,
}

#[derive(Clone)]
pub struct RelayClient {
    connection: TransportConnection,
    shared: Arc<RelayShared>,
    #[allow(dead_code)]
    next_request_id: Arc<AtomicU64>,
}

impl RelayClient {
    pub fn preview_invite(invite_link: &str) -> Result<InvitePreview, String> {
        let claims = parse_unsigned_invite_claims(invite_link)?;
        Ok(invite_preview_from_claims(&claims))
    }

    pub async fn join_with_invite(
        invite_link: &str,
        registration: DeviceRegistrationBundle,
    ) -> Result<JoinAccepted, String> {
        let claims = parse_unsigned_invite_claims(invite_link)?;
        let trusted_server_certificate_der = decode_invite_certificate(&claims)?;

        let endpoint_config = TransportEndpointConfig::new(
            SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            claims.server_name.clone(),
        );
        let endpoint = TransportEndpoint::bind(
            endpoint_config.clone(),
            TransportIdentity::generate(endpoint_config.server_name.clone())
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let connection = endpoint
            .connect(
                claims
                    .server_addr
                    .parse()
                    .map_err(|error| format!("invalid invite server addr: {error}"))?,
                &trusted_server_certificate_der,
                &ReconnectPolicy::new(3, Duration::from_millis(50), Duration::from_millis(250)),
            )
            .await
            .map_err(|error| error.to_string())?;

        send_client_envelope(
            &connection,
            &ClientEnvelope::JoinWithInvite(JoinWithInvite {
                invite_link: invite_link.to_string(),
                registration,
            }),
        )
        .await?;

        match receive_server_envelope(&connection).await? {
            ServerEnvelope::JoinAccepted(accepted) => Ok(accepted),
            ServerEnvelope::Disconnect(disconnect) => Err(disconnect.reason),
            other => Err(format!(
                "expected join accepted from relay server, got {other:?}"
            )),
        }
    }

    pub async fn connect(
        config: &RelayClientConfig,
        auth_device: &Device,
        identity_material: &IdentityKeyMaterial,
    ) -> Result<RelayBootstrap, String> {
        let endpoint_config = TransportEndpointConfig::new(
            SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            config.server_name.clone(),
        );
        let endpoint = TransportEndpoint::bind(
            endpoint_config.clone(),
            TransportIdentity::generate(endpoint_config.server_name.clone())
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let connection = endpoint
            .connect(
                config.server_addr,
                &config.trusted_server_certificate_der,
                &ReconnectPolicy::new(3, Duration::from_millis(50), Duration::from_millis(250)),
            )
            .await
            .map_err(|error| error.to_string())?;

        send_client_envelope(
            &connection,
            &ClientEnvelope::AuthHello(AuthHello {
                version: SERVER_PROTOCOL_VERSION,
                member_id: auth_device.owner_member_id().to_string(),
                device_id: auth_device.device_id().to_string(),
            }),
        )
        .await?;

        let challenge = match receive_server_envelope(&connection).await? {
            ServerEnvelope::AuthChallenge(challenge) => challenge,
            other => {
                return Err(format!(
                    "expected auth challenge from relay server, got {other:?}"
                ));
            }
        };

        let identity = IdentityKeyPair::from_material(identity_material);
        send_client_envelope(
            &connection,
            &ClientEnvelope::AuthResponse(AuthResponse {
                version: SERVER_PROTOCOL_VERSION,
                member_id: auth_device.owner_member_id().to_string(),
                device_id: auth_device.device_id().to_string(),
                nonce: challenge.nonce,
                signature: identity
                    .sign_message(&auth_challenge_payload(
                        auth_device.owner_member_id().as_str(),
                        auth_device.device_id().as_str(),
                        &challenge.nonce,
                    ))
                    .to_vec(),
            }),
        )
        .await?;

        match receive_server_envelope(&connection).await? {
            ServerEnvelope::AuthOk(_) => {
                let client = Self {
                    connection: connection.clone(),
                    shared: Arc::new(RelayShared {
                        pending_route_results: Mutex::new(HashMap::new()),
                        pending_blob_starts: Mutex::new(HashMap::new()),
                        pending_blob_chunks: Mutex::new(HashMap::new()),
                        pending_blob_finishes: Mutex::new(HashMap::new()),
                        pending_blob_downloads: Mutex::new(HashMap::new()),
                        inboxes: Mutex::new(HashMap::new()),
                    }),
                    next_request_id: Arc::new(AtomicU64::new(1)),
                };
                client.spawn_receiver();
                Ok(RelayBootstrap {
                    client,
                    server_status: RelayServerStatus::Connected,
                    auth_status: RelayAuthStatus::Authenticated,
                })
            }
            ServerEnvelope::Disconnect(disconnect) => Err(disconnect.reason),
            other => Err(format!("expected auth ok from relay server, got {other:?}")),
        }
    }

    pub async fn health_check(&self) -> Result<(), String> {
        send_client_envelope(&self.connection, &ClientEnvelope::HealthCheck).await
    }

    pub async fn upload_blob(
        &self,
        file_name: impl Into<String>,
        mime_type: impl Into<String>,
        media_kind: localmessenger_server_protocol::MediaKind,
        plaintext_bytes: u64,
        ciphertext: Vec<u8>,
        sha256_hex: String,
    ) -> Result<UploadedBlobHandle, String> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (ready_tx, ready_rx) = oneshot::channel();
        self.shared
            .pending_blob_starts
            .lock()
            .await
            .insert(request_id, ready_tx);
        let start = BlobUploadStart {
            request_id,
            file_name: file_name.into(),
            mime_type: mime_type.into(),
            media_kind,
            plaintext_bytes,
            ciphertext_bytes: ciphertext.len() as u64,
            sha256_hex,
        };
        send_client_envelope(&self.connection, &ClientEnvelope::BlobUploadStart(start)).await?;
        let ready = tokio::time::timeout(Duration::from_secs(2), ready_rx)
            .await
            .map_err(|_| "blob upload start timed out".to_string())?
            .map_err(|_| "blob upload start response channel closed".to_string())??;

        let mut offset = 0_u64;
        for chunk in ciphertext.chunks(MAX_BLOB_CHUNK_BYTES) {
            let (ack_tx, ack_rx) = oneshot::channel();
            self.shared
                .pending_blob_chunks
                .lock()
                .await
                .insert(request_id, ack_tx);
            send_client_envelope(
                &self.connection,
                &ClientEnvelope::BlobUploadChunk(BlobUploadChunk {
                    request_id,
                    blob_id: ready.blob_id.clone(),
                    offset,
                    bytes: chunk.to_vec(),
                }),
            )
            .await?;
            let ack = tokio::time::timeout(Duration::from_secs(2), ack_rx)
                .await
                .map_err(|_| "blob chunk ack timed out".to_string())?
                .map_err(|_| "blob chunk ack channel closed".to_string())??;
            if ack.received_bytes != offset + chunk.len() as u64 {
                return Err("blob chunk ack returned unexpected byte count".to_string());
            }
            offset = ack.received_bytes;
        }

        let (finish_tx, finish_rx) = oneshot::channel();
        self.shared
            .pending_blob_finishes
            .lock()
            .await
            .insert(request_id, finish_tx);
        send_client_envelope(
            &self.connection,
            &ClientEnvelope::BlobUploadFinish(BlobUploadFinish {
                request_id,
                blob_id: ready.blob_id,
                ciphertext_bytes: ciphertext.len() as u64,
            }),
        )
        .await?;
        let stored = tokio::time::timeout(Duration::from_secs(2), finish_rx)
            .await
            .map_err(|_| "blob upload finish timed out".to_string())?
            .map_err(|_| "blob upload finish channel closed".to_string())??;
        Ok(UploadedBlobHandle {
            metadata: stored.blob,
        })
    }

    pub async fn download_blob(&self, blob_id: &str) -> Result<DownloadedBlob, String> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.shared
            .pending_blob_downloads
            .lock()
            .await
            .insert(request_id, tx);
        send_client_envelope(
            &self.connection,
            &ClientEnvelope::BlobDownloadRequest(BlobDownloadRequest {
                request_id,
                blob_id: blob_id.to_string(),
            }),
        )
        .await?;

        let mut metadata: Option<StoredBlob> = None;
        let mut ciphertext = Vec::new();
        loop {
            let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .map_err(|_| "blob download timed out".to_string())?
                .ok_or_else(|| "blob download channel closed".to_string())?;
            match event {
                BlobDownloadEvent::Ready(ready) => {
                    metadata = Some(ready.blob);
                }
                BlobDownloadEvent::Chunk(chunk) => {
                    if chunk.offset != ciphertext.len() as u64 {
                        self.shared
                            .pending_blob_downloads
                            .lock()
                            .await
                            .remove(&request_id);
                        return Err("blob download chunks arrived out of order".to_string());
                    }
                    ciphertext.extend_from_slice(&chunk.bytes);
                    if chunk.is_last {
                        break;
                    }
                }
                BlobDownloadEvent::Rejected(reason) => {
                    self.shared
                        .pending_blob_downloads
                        .lock()
                        .await
                        .remove(&request_id);
                    return Err(reason);
                }
            }
        }
        self.shared
            .pending_blob_downloads
            .lock()
            .await
            .remove(&request_id);
        let metadata = metadata.ok_or_else(|| "blob download missing metadata".to_string())?;
        if ciphertext.len() as u64 != metadata.ciphertext_bytes {
            return Err("blob download byte count does not match metadata".to_string());
        }
        Ok(DownloadedBlob {
            metadata,
            ciphertext,
        })
    }

    #[allow(dead_code)]
    pub async fn peer_channel(&self, remote_device_id: &DeviceId) -> RelayPeerChannel {
        let (tx, rx) = mpsc::unbounded_channel();
        self.shared
            .inboxes
            .lock()
            .await
            .insert(remote_device_id.to_string(), tx);
        RelayPeerChannel {
            client: self.clone(),
            remote_device_id: remote_device_id.to_string(),
            inbox: Mutex::new(rx),
        }
    }

    fn spawn_receiver(&self) {
        let connection = self.connection.clone();
        let shared = self.shared.clone();
        tokio::spawn(async move {
            while let Ok(envelope) = receive_server_envelope(&connection).await {
                match envelope {
                    ServerEnvelope::PeerFrame(frame) => {
                        if let Ok(transport_frame) =
                            bincode::deserialize::<TransportFrame>(&frame.payload)
                        {
                            let maybe_sender = {
                                shared
                                    .inboxes
                                    .lock()
                                    .await
                                    .get(&frame.sender_device_id)
                                    .cloned()
                            };
                            if let Some(sender) = maybe_sender {
                                let _ = sender.send(transport_frame);
                            }
                        }
                    }
                    ServerEnvelope::PeerUnavailable(unavailable) => {
                        if let Some(waiter) = shared
                            .pending_route_results
                            .lock()
                            .await
                            .remove(&unavailable.request_id)
                        {
                            let _ = waiter.send(RouteSendResult::Unavailable(unavailable.reason));
                        }
                    }
                    ServerEnvelope::PeerQueued(queued) => {
                        if let Some(waiter) = shared
                            .pending_route_results
                            .lock()
                            .await
                            .remove(&queued.request_id)
                        {
                            let _ = waiter.send(RouteSendResult::Queued);
                        }
                    }
                    ServerEnvelope::BlobUploadReady(ready) => {
                        if let Some(waiter) = shared
                            .pending_blob_starts
                            .lock()
                            .await
                            .remove(&ready.request_id)
                        {
                            let _ = waiter.send(Ok(ready));
                        }
                    }
                    ServerEnvelope::BlobChunkAck(ack) => {
                        if let Some(waiter) = shared
                            .pending_blob_chunks
                            .lock()
                            .await
                            .remove(&ack.request_id)
                        {
                            let _ = waiter.send(Ok(ack));
                        }
                    }
                    ServerEnvelope::BlobStored(stored) => {
                        if let Some(waiter) = shared
                            .pending_blob_finishes
                            .lock()
                            .await
                            .remove(&stored.request_id)
                        {
                            let _ = waiter.send(Ok(stored));
                        }
                    }
                    ServerEnvelope::BlobDownloadReady(ready) => {
                        if let Some(sender) = shared
                            .pending_blob_downloads
                            .lock()
                            .await
                            .get(&ready.request_id)
                            .cloned()
                        {
                            let _ = sender.send(BlobDownloadEvent::Ready(ready));
                        }
                    }
                    ServerEnvelope::BlobDownloadChunk(chunk) => {
                        if let Some(sender) = shared
                            .pending_blob_downloads
                            .lock()
                            .await
                            .get(&chunk.request_id)
                            .cloned()
                        {
                            let _ = sender.send(BlobDownloadEvent::Chunk(chunk));
                        }
                    }
                    ServerEnvelope::BlobRejected(rejected) => {
                        route_blob_rejection(&shared, rejected).await;
                    }
                    ServerEnvelope::Health(_)
                    | ServerEnvelope::Disconnect(_)
                    | ServerEnvelope::JoinAccepted(_) => {}
                    ServerEnvelope::AuthChallenge(_) | ServerEnvelope::AuthOk(_) => {}
                }
            }
        });
    }
}

#[allow(dead_code)]
pub struct RelayPeerChannel {
    client: RelayClient,
    remote_device_id: String,
    inbox: Mutex<mpsc::UnboundedReceiver<TransportFrame>>,
}

impl FrameChannel for RelayPeerChannel {
    fn send_frame<'a>(
        &'a self,
        frame: &'a TransportFrame,
    ) -> Pin<Box<dyn Future<Output = Result<(), TransportError>> + Send + 'a>> {
        Box::pin(async move {
            let request_id = self.client.next_request_id.fetch_add(1, Ordering::Relaxed);
            let (tx, rx) = oneshot::channel();
            self.client
                .shared
                .pending_route_results
                .lock()
                .await
                .insert(request_id, tx);

            let envelope =
                ClientEnvelope::PeerRelayFrame(localmessenger_server_protocol::PeerRelayFrame {
                    request_id,
                    recipient_device_id: self.remote_device_id.clone(),
                    payload: bincode::serialize(frame)
                        .map_err(|error| TransportError::FrameEncoding(error.to_string()))?,
                });
            send_client_envelope(&self.client.connection, &envelope)
                .await
                .map_err(TransportError::Connect)?;

            match tokio::time::timeout(Duration::from_millis(25), rx).await {
                Ok(Ok(RouteSendResult::Queued)) => Ok(()),
                Ok(Ok(RouteSendResult::Unavailable(reason))) => {
                    Err(TransportError::Connect(format!(
                        "relay recipient unavailable: {}",
                        peer_unavailable_reason_label(reason)
                    )))
                }
                Ok(Err(_)) | Err(_) => {
                    self.client
                        .shared
                        .pending_route_results
                        .lock()
                        .await
                        .remove(&request_id);
                    Ok(())
                }
            }
        })
    }

    fn receive_frame<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<TransportFrame, TransportError>> + Send + 'a>> {
        Box::pin(async move {
            self.inbox
                .lock()
                .await
                .recv()
                .await
                .ok_or(TransportError::ConnectionClosed)
        })
    }

    fn close(&self, _reason: &'static str) {}
}

pub fn resolve_active_route(
    preferred_routes: &[TransportRoute],
    relay_ready: bool,
) -> TransportRoute {
    for route in preferred_routes {
        match route {
            TransportRoute::ServerRelay if relay_ready => return TransportRoute::ServerRelay,
            TransportRoute::DirectLan => return TransportRoute::DirectLan,
            TransportRoute::ServerRelay => {}
        }
    }
    TransportRoute::DirectLan
}

fn parse_transport_order(raw: String) -> Vec<TransportRoute> {
    let mut routes = Vec::new();
    for value in raw.split(',') {
        match value.trim() {
            "server_relay" => routes.push(TransportRoute::ServerRelay),
            "direct_lan" => routes.push(TransportRoute::DirectLan),
            _ => {}
        }
    }
    if routes.is_empty() {
        routes.push(TransportRoute::DirectLan);
    }
    if !routes.contains(&TransportRoute::DirectLan) {
        routes.push(TransportRoute::DirectLan);
    }
    routes
}

fn parse_unsigned_invite_claims(
    invite_link: &str,
) -> Result<localmessenger_server_protocol::InviteClaims, String> {
    let token = localmessenger_server_protocol::parse_invite_link(invite_link)?;
    let (payload_segment, _) = token
        .split_once('.')
        .ok_or_else(|| "invite token must contain payload and signature".to_string())?;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_segment)
        .map_err(|error| error.to_string())?;
    let claims: localmessenger_server_protocol::InviteClaims =
        serde_json::from_slice(&payload).map_err(|error| error.to_string())?;
    claims.validate()?;
    Ok(claims)
}

fn decode_join_accepted_certificate(encoded: &str) -> Result<Vec<u8>, String> {
    let claims = localmessenger_server_protocol::InviteClaims {
        version: SERVER_PROTOCOL_VERSION,
        invite_id: "joined".to_string(),
        label: "joined".to_string(),
        server_addr: "0.0.0.0:0".to_string(),
        server_name: "relay.local".to_string(),
        server_certificate_der_base64: encoded.to_string(),
        issued_at_unix_ms: 0,
        expires_at_unix_ms: 1,
        max_uses: 1,
    };
    decode_invite_certificate(&claims)
}

#[allow(dead_code)]
fn peer_unavailable_reason_label(reason: PeerUnavailableReason) -> &'static str {
    match reason {
        PeerUnavailableReason::Offline => "offline",
        PeerUnavailableReason::UnknownRecipient => "unknown_recipient",
        PeerUnavailableReason::Disabled => "disabled",
        PeerUnavailableReason::Unauthorized => "unauthorized",
        PeerUnavailableReason::RateLimited => "rate_limited",
    }
}

async fn route_blob_rejection(shared: &RelayShared, rejected: BlobRejected) {
    if let Some(waiter) = shared
        .pending_blob_starts
        .lock()
        .await
        .remove(&rejected.request_id)
    {
        let _ = waiter.send(Err(rejected.reason));
        return;
    }
    if let Some(waiter) = shared
        .pending_blob_chunks
        .lock()
        .await
        .remove(&rejected.request_id)
    {
        let _ = waiter.send(Err(rejected.reason));
        return;
    }
    if let Some(waiter) = shared
        .pending_blob_finishes
        .lock()
        .await
        .remove(&rejected.request_id)
    {
        let _ = waiter.send(Err(rejected.reason));
        return;
    }
    if let Some(sender) = shared
        .pending_blob_downloads
        .lock()
        .await
        .get(&rejected.request_id)
        .cloned()
    {
        let _ = sender.send(BlobDownloadEvent::Rejected(rejected.reason));
    }
}

async fn send_client_envelope(
    connection: &TransportConnection,
    envelope: &ClientEnvelope,
) -> Result<(), String> {
    connection
        .send_frame(&TransportFrame::payload(
            bincode::serialize(envelope).map_err(|error| error.to_string())?,
        ))
        .await
        .map_err(|error| error.to_string())
}

async fn receive_server_envelope(
    connection: &TransportConnection,
) -> Result<ServerEnvelope, String> {
    let frame = connection
        .receive_frame()
        .await
        .map_err(|error| error.to_string())?;
    match frame {
        TransportFrame::Payload(bytes) => {
            bincode::deserialize(&bytes).map_err(|error| error.to_string())
        }
        _ => Err("expected payload transport frame from relay server".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{RelayClient, TransportRoute, resolve_active_route};
    use base64::Engine;
    use localmessenger_server_protocol::{
        INVITE_LINK_PREFIX, InviteClaims, SERVER_PROTOCOL_VERSION, encode_invite_link,
    };

    #[test]
    fn route_selection_prefers_healthy_server_relay_and_falls_back_to_direct() {
        assert_eq!(
            resolve_active_route(
                &[TransportRoute::ServerRelay, TransportRoute::DirectLan],
                true
            ),
            TransportRoute::ServerRelay
        );
        assert_eq!(
            resolve_active_route(
                &[TransportRoute::ServerRelay, TransportRoute::DirectLan],
                false
            ),
            TransportRoute::DirectLan
        );
    }

    #[test]
    fn invite_preview_extracts_server_details_from_link() {
        let claims = InviteClaims {
            version: SERVER_PROTOCOL_VERSION,
            invite_id: "inv-42".to_string(),
            label: "Home relay".to_string(),
            server_addr: "127.0.0.1:7443".to_string(),
            server_name: "relay.local".to_string(),
            server_certificate_der_base64: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode([1_u8; 4]),
            issued_at_unix_ms: 10,
            expires_at_unix_ms: 20,
            max_uses: 2,
        };
        let link = encode_invite_link(b"secret", &claims).expect("link");
        assert!(link.starts_with(INVITE_LINK_PREFIX));
        let preview = RelayClient::preview_invite(&link).expect("preview");
        assert_eq!(preview.server_name, "relay.local");
        assert_eq!(preview.invite_id, "inv-42");
    }
}
