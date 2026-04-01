#![forbid(unsafe_code)]

mod auth;
mod invite;
mod rate_limit;
mod registry;
mod relay;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use auth::Authenticator;
use invite::InviteService;
use localmessenger_server_protocol::{
    BlobChunkAck, BlobDownloadChunk, BlobDownloadReady, BlobRejected, BlobStored, BlobUploadReady,
    ClientEnvelope, DeviceRegistrationBundle, Disconnect, Health, JoinWithInvite,
    MAX_BLOB_CHUNK_BYTES, MAX_RELAY_BLOB_BYTES, SERVER_PROTOCOL_VERSION, ServerEnvelope,
    StoredBlob,
};
use localmessenger_transport::{
    TransportConnection, TransportEndpoint, TransportEndpointConfig, TransportFrame,
    TransportIdentity,
};
use rand_core::OsRng;
use rate_limit::{RateLimitBucket, RateLimitProfile, RateLimiter};
use registry::RegistryDatabase;
use relay::{RelayState, RoutePeerFrameOutcome, queued_notice};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
struct PendingBlobUpload {
    blob_id: String,
    file_name: String,
    mime_type: String,
    media_kind: localmessenger_server_protocol::MediaKind,
    plaintext_bytes: u64,
    ciphertext_bytes: u64,
    sha256_hex: String,
    buffer: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("serve") => {
            let command = ServeCommand::from_args(&args[2..])?;
            run_server(command).await
        }
        Some("register-device") => {
            let command = RegisterDeviceCommand::from_args(&args[2..])?;
            register_device(command).await
        }
        Some("disable-device") => {
            let command = DisableDeviceCommand::from_args(&args[2..])?;
            disable_device(command).await
        }
        Some("create-invite") => {
            let command = CreateInviteCommand::from_args(&args[2..])?;
            create_invite(command).await
        }
        Some("list-invites") => {
            let command = DatabaseCommand::from_args(&args[2..])?;
            list_invites(command).await
        }
        Some("list-devices") => {
            let command = DatabaseCommand::from_args(&args[2..])?;
            list_devices(command).await
        }
        _ => {
            print_help();
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
struct ServeCommand {
    bind_addr: SocketAddr,
    server_name: String,
    cert_path: PathBuf,
    key_path: PathBuf,
    database_url: String,
    invite_secret: String,
    rate_limit_window_ms: i64,
    peer_frame_limit: u64,
    blob_request_limit: u64,
    blob_chunk_byte_limit: u64,
    health_check_limit: u64,
}

impl ServeCommand {
    fn from_args(args: &[String]) -> Result<Self, String> {
        Ok(Self {
            bind_addr: parse_flag(args, "--bind")?
                .parse()
                .map_err(|error| format!("invalid --bind: {error}"))?,
            server_name: parse_flag(args, "--server-name")?,
            cert_path: PathBuf::from(parse_flag(args, "--cert")?),
            key_path: PathBuf::from(parse_flag(args, "--key")?),
            database_url: parse_flag(args, "--db")?,
            invite_secret: parse_flag(args, "--invite-secret")?,
            rate_limit_window_ms: parse_optional_flag(args, "--rate-window-seconds")
                .unwrap_or_else(|| "60".to_string())
                .parse::<i64>()
                .map_err(|error| format!("invalid --rate-window-seconds: {error}"))?
                .saturating_mul(1000),
            peer_frame_limit: parse_optional_flag(args, "--peer-frame-limit")
                .unwrap_or_else(|| "120".to_string())
                .parse()
                .map_err(|error| format!("invalid --peer-frame-limit: {error}"))?,
            blob_request_limit: parse_optional_flag(args, "--blob-request-limit")
                .unwrap_or_else(|| "32".to_string())
                .parse()
                .map_err(|error| format!("invalid --blob-request-limit: {error}"))?,
            blob_chunk_byte_limit: parse_optional_flag(args, "--blob-chunk-byte-limit")
                .unwrap_or_else(|| (20_u64 * 1024 * 1024).to_string())
                .parse()
                .map_err(|error| format!("invalid --blob-chunk-byte-limit: {error}"))?,
            health_check_limit: parse_optional_flag(args, "--health-check-limit")
                .unwrap_or_else(|| "12".to_string())
                .parse()
                .map_err(|error| format!("invalid --health-check-limit: {error}"))?,
        })
    }
}

#[derive(Debug, Clone)]
struct RegisterDeviceCommand {
    database_url: String,
    bundle_path: PathBuf,
}

impl RegisterDeviceCommand {
    fn from_args(args: &[String]) -> Result<Self, String> {
        Ok(Self {
            database_url: parse_flag(args, "--db")?,
            bundle_path: PathBuf::from(parse_flag(args, "--bundle")?),
        })
    }
}

#[derive(Debug, Clone)]
struct DisableDeviceCommand {
    database_url: String,
    device_id: String,
}

impl DisableDeviceCommand {
    fn from_args(args: &[String]) -> Result<Self, String> {
        Ok(Self {
            database_url: parse_flag(args, "--db")?,
            device_id: parse_flag(args, "--device-id")?,
        })
    }
}

#[derive(Debug, Clone)]
struct DatabaseCommand {
    database_url: String,
}

impl DatabaseCommand {
    fn from_args(args: &[String]) -> Result<Self, String> {
        Ok(Self {
            database_url: parse_flag(args, "--db")?,
        })
    }
}

#[derive(Debug, Clone)]
struct CreateInviteCommand {
    database_url: String,
    invite_secret: String,
    invite_id: String,
    label: String,
    server_addr: String,
    server_name: String,
    cert_path: PathBuf,
    ttl_seconds: i64,
    max_uses: u32,
}

impl CreateInviteCommand {
    fn from_args(args: &[String]) -> Result<Self, String> {
        let invite_id = match parse_optional_flag(args, "--invite-id") {
            Some(value) => value,
            None => format!("invite-{}", now_unix_ms()),
        };

        Ok(Self {
            database_url: parse_flag(args, "--db")?,
            invite_secret: parse_flag(args, "--invite-secret")?,
            invite_id,
            label: parse_flag(args, "--label")?,
            server_addr: parse_flag(args, "--server-addr")?,
            server_name: parse_flag(args, "--server-name")?,
            cert_path: PathBuf::from(parse_flag(args, "--cert")?),
            ttl_seconds: parse_flag(args, "--ttl-seconds")?
                .parse()
                .map_err(|error| format!("invalid --ttl-seconds: {error}"))?,
            max_uses: parse_flag(args, "--max-uses")?
                .parse()
                .map_err(|error| format!("invalid --max-uses: {error}"))?,
        })
    }
}

async fn run_server(command: ServeCommand) -> Result<(), String> {
    let registry = RegistryDatabase::open(&command.database_url).await?;
    let relay = RelayState::new(registry.clone());
    let authenticator = Authenticator::new(30_000);
    let invite_service = InviteService::new(registry.clone(), command.invite_secret.into_bytes());
    let rate_limiter = RateLimiter::new(RateLimitProfile::new(
        command.rate_limit_window_ms,
        command.peer_frame_limit,
        command.blob_request_limit,
        command.blob_chunk_byte_limit,
        command.health_check_limit,
    )?);

    let identity =
        load_transport_identity(&command.server_name, &command.cert_path, &command.key_path)?;
    let endpoint = TransportEndpoint::bind(
        TransportEndpointConfig::new(command.bind_addr, command.server_name),
        identity,
    )
    .map_err(|error| error.to_string())?;

    println!("Local Messenger relay listening on {}", command.bind_addr);

    loop {
        let connection = endpoint.accept().await.map_err(|error| error.to_string())?;
        let relay = relay.clone();
        let registry = registry.clone();
        let authenticator = authenticator.clone();
        let invite_service = invite_service.clone();
        let rate_limiter = rate_limiter.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(
                connection,
                relay,
                registry,
                authenticator,
                invite_service,
                rate_limiter,
            )
            .await
            {
                eprintln!("relay session ended with error: {error}");
            }
        });
    }
}

async fn handle_connection(
    connection: TransportConnection,
    relay: RelayState,
    registry: RegistryDatabase,
    authenticator: Authenticator,
    invite_service: InviteService,
    rate_limiter: RateLimiter,
) -> Result<(), String> {
    let first_envelope = receive_client_envelope(&connection).await?;
    if let ClientEnvelope::JoinWithInvite(join) = first_envelope {
        return handle_join_with_invite(connection, invite_service, join).await;
    }

    let hello = match first_envelope {
        ClientEnvelope::AuthHello(hello) => hello,
        _ => {
            send_server_envelope(
                &connection,
                &ServerEnvelope::Disconnect(Disconnect {
                    reason: "expected auth hello or join request".to_string(),
                }),
            )
            .await?;
            return Err("expected auth hello or join request".to_string());
        }
    };

    let record = registry
        .registered_device(&hello.device_id)
        .await?
        .ok_or_else(|| "unregistered device".to_string())?;
    let mut rng = OsRng;
    let mut challenge = authenticator.issue_challenge(&mut rng, now_unix_ms());
    send_server_envelope(
        &connection,
        &ServerEnvelope::AuthChallenge(challenge.challenge.clone()),
    )
    .await?;

    let response = match receive_client_envelope(&connection).await? {
        ClientEnvelope::AuthResponse(response) => response,
        _ => return Err("expected auth response".to_string()),
    };

    let auth_ok =
        authenticator.verify_response(&hello, &response, &record, &mut challenge, now_unix_ms())?;
    send_server_envelope(&connection, &ServerEnvelope::AuthOk(auth_ok)).await?;
    registry
        .touch_last_seen(&hello.device_id, now_unix_ms())
        .await?;

    for (row_id, envelope) in relay
        .queued_envelopes_for_recipient(&hello.device_id)
        .await?
    {
        send_server_envelope(&connection, &envelope).await?;
        relay.delete_queued_frame(row_id).await?;
    }

    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel();
    relay
        .register_online(hello.device_id.clone(), outbound_tx.clone())
        .await;

    let writer_connection = connection.clone();
    let writer_task = tokio::spawn(async move {
        while let Some(envelope) = outbound_rx.recv().await {
            if send_server_envelope(&writer_connection, &envelope)
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let result = async {
        let mut pending_uploads = HashMap::<u64, PendingBlobUpload>::new();
        loop {
            match receive_client_envelope(&connection).await? {
                ClientEnvelope::PeerRelayFrame(frame) => {
                    let queued_at = now_unix_ms();
                    if !rate_limiter
                        .allow(&hello.device_id, RateLimitBucket::PeerFrame, 1, queued_at)
                        .await
                    {
                        let _ = outbound_tx.send(ServerEnvelope::PeerUnavailable(
                            localmessenger_server_protocol::PeerUnavailable {
                                request_id: frame.request_id,
                                recipient_device_id: frame.recipient_device_id,
                                reason: localmessenger_server_protocol::PeerUnavailableReason::RateLimited,
                            },
                        ));
                        continue;
                    }
                    match relay
                        .route_peer_frame(&hello.device_id, frame.clone(), queued_at)
                        .await
                    {
                        Ok(RoutePeerFrameOutcome::Delivered) => {}
                        Ok(RoutePeerFrameOutcome::Queued) => {
                            let _ = outbound_tx.send(queued_notice(&frame, queued_at));
                        }
                        Err(unavailable) => {
                            let _ = outbound_tx.send(ServerEnvelope::PeerUnavailable(unavailable));
                        }
                    }
                }
                ClientEnvelope::HealthCheck => {
                    let now = now_unix_ms();
                    if !rate_limiter
                        .allow(&hello.device_id, RateLimitBucket::HealthCheck, 1, now)
                        .await
                    {
                        let _ = outbound_tx.send(ServerEnvelope::Disconnect(Disconnect {
                            reason: "rate limit exceeded for health checks".to_string(),
                        }));
                        break;
                    }
                    let _ = outbound_tx.send(ServerEnvelope::Health(Health {
                        version: SERVER_PROTOCOL_VERSION,
                        server_time_unix_ms: now,
                        online_devices: relay.online_count().await as u32,
                    }));
                }
                ClientEnvelope::BlobUploadStart(start) => {
                    if !rate_limiter
                        .allow(
                            &hello.device_id,
                            RateLimitBucket::BlobRequest,
                            1,
                            now_unix_ms(),
                        )
                        .await
                    {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: start.request_id,
                            reason: "rate limit exceeded for blob requests".to_string(),
                        }));
                        continue;
                    }
                    if let Err(reason) = start.validate() {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: start.request_id,
                            reason,
                        }));
                        continue;
                    }

                    let blob_id = format!("blob-{}-{}", hello.device_id, start.request_id);
                    pending_uploads.insert(
                        start.request_id,
                        PendingBlobUpload {
                            blob_id: blob_id.clone(),
                            file_name: start.file_name,
                            mime_type: start.mime_type,
                            media_kind: start.media_kind,
                            plaintext_bytes: start.plaintext_bytes,
                            ciphertext_bytes: start.ciphertext_bytes,
                            sha256_hex: start.sha256_hex,
                            buffer: Vec::with_capacity(start.ciphertext_bytes as usize),
                        },
                    );
                    let _ = outbound_tx.send(ServerEnvelope::BlobUploadReady(BlobUploadReady {
                        request_id: start.request_id,
                        blob_id,
                        max_chunk_bytes: MAX_BLOB_CHUNK_BYTES as u32,
                    }));
                }
                ClientEnvelope::BlobUploadChunk(chunk) => {
                    let Some(upload) = pending_uploads.get_mut(&chunk.request_id) else {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: chunk.request_id,
                            reason: "unknown blob upload request".to_string(),
                        }));
                        continue;
                    };
                    if upload.blob_id != chunk.blob_id {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: chunk.request_id,
                            reason: "blob id does not match pending upload".to_string(),
                        }));
                        continue;
                    }
                    if chunk.bytes.len() > MAX_BLOB_CHUNK_BYTES {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: chunk.request_id,
                            reason: format!(
                                "chunk exceeds maximum size of {} bytes",
                                MAX_BLOB_CHUNK_BYTES
                            ),
                        }));
                        continue;
                    }
                    if !rate_limiter
                        .allow(
                            &hello.device_id,
                            RateLimitBucket::BlobChunkBytes,
                            chunk.bytes.len() as u64,
                            now_unix_ms(),
                        )
                        .await
                    {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: chunk.request_id,
                            reason: "rate limit exceeded for relay blob bandwidth".to_string(),
                        }));
                        continue;
                    }
                    if chunk.offset != upload.buffer.len() as u64 {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: chunk.request_id,
                            reason: "chunk offset is out of order".to_string(),
                        }));
                        continue;
                    }
                    let next_size = upload.buffer.len() as u64 + chunk.bytes.len() as u64;
                    if next_size > MAX_RELAY_BLOB_BYTES || next_size > upload.ciphertext_bytes {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: chunk.request_id,
                            reason: "blob upload exceeds negotiated size".to_string(),
                        }));
                        continue;
                    }
                    upload.buffer.extend_from_slice(&chunk.bytes);
                    let _ = outbound_tx.send(ServerEnvelope::BlobChunkAck(BlobChunkAck {
                        request_id: chunk.request_id,
                        blob_id: upload.blob_id.clone(),
                        received_bytes: upload.buffer.len() as u64,
                    }));
                }
                ClientEnvelope::BlobUploadFinish(finish) => {
                    let Some(upload) = pending_uploads.remove(&finish.request_id) else {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: finish.request_id,
                            reason: "unknown blob upload request".to_string(),
                        }));
                        continue;
                    };
                    if upload.blob_id != finish.blob_id {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: finish.request_id,
                            reason: "blob id does not match pending upload".to_string(),
                        }));
                        continue;
                    }
                    if upload.buffer.len() as u64 != finish.ciphertext_bytes
                        || finish.ciphertext_bytes != upload.ciphertext_bytes
                    {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: finish.request_id,
                            reason: "blob upload is incomplete".to_string(),
                        }));
                        continue;
                    }

                    let stored = StoredBlob {
                        blob_id: upload.blob_id.clone(),
                        uploaded_by_device_id: hello.device_id.clone(),
                        file_name: upload.file_name,
                        mime_type: upload.mime_type,
                        media_kind: upload.media_kind,
                        plaintext_bytes: upload.plaintext_bytes,
                        ciphertext_bytes: upload.ciphertext_bytes,
                        sha256_hex: upload.sha256_hex,
                        created_at_unix_ms: now_unix_ms(),
                    };
                    if let Err(reason) = registry.store_blob(&stored, &upload.buffer).await {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: finish.request_id,
                            reason,
                        }));
                        continue;
                    }
                    let _ = outbound_tx.send(ServerEnvelope::BlobStored(BlobStored {
                        request_id: finish.request_id,
                        blob: stored,
                    }));
                }
                ClientEnvelope::BlobDownloadRequest(request) => {
                    if !rate_limiter
                        .allow(
                            &hello.device_id,
                            RateLimitBucket::BlobRequest,
                            1,
                            now_unix_ms(),
                        )
                        .await
                    {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: request.request_id,
                            reason: "rate limit exceeded for blob requests".to_string(),
                        }));
                        continue;
                    }
                    let Some(stored) = registry.blob(&request.blob_id).await? else {
                        let _ = outbound_tx.send(ServerEnvelope::BlobRejected(BlobRejected {
                            request_id: request.request_id,
                            reason: format!("blob '{}' was not found", request.blob_id),
                        }));
                        continue;
                    };
                    let _ =
                        outbound_tx.send(ServerEnvelope::BlobDownloadReady(BlobDownloadReady {
                            request_id: request.request_id,
                            blob: stored.metadata.clone(),
                            max_chunk_bytes: MAX_BLOB_CHUNK_BYTES as u32,
                        }));
                    let chunk_size = MAX_BLOB_CHUNK_BYTES;
                    let total = stored.ciphertext.len();
                    for (index, chunk) in stored.ciphertext.chunks(chunk_size).enumerate() {
                        let offset = index.saturating_mul(chunk_size) as u64;
                        let is_last = offset + chunk.len() as u64 >= total as u64;
                        let _ = outbound_tx.send(ServerEnvelope::BlobDownloadChunk(
                            BlobDownloadChunk {
                                request_id: request.request_id,
                                blob_id: stored.metadata.blob_id.clone(),
                                offset,
                                bytes: chunk.to_vec(),
                                is_last,
                            },
                        ));
                    }
                }
                ClientEnvelope::Disconnect(_) => break,
                ClientEnvelope::JoinWithInvite(_)
                | ClientEnvelope::AuthHello(_)
                | ClientEnvelope::AuthResponse(_) => {
                    return Err("unexpected auth envelope after session start".to_string());
                }
            }
        }
        Ok(())
    }
    .await;

    relay.unregister_online(&hello.device_id).await;
    writer_task.abort();
    result
}

async fn handle_join_with_invite(
    connection: TransportConnection,
    invite_service: InviteService,
    join: JoinWithInvite,
) -> Result<(), String> {
    match invite_service.join_with_invite(&join, now_unix_ms()).await {
        Ok(accepted) => {
            send_server_envelope(&connection, &ServerEnvelope::JoinAccepted(accepted)).await?;
            Ok(())
        }
        Err(reason) => {
            send_server_envelope(
                &connection,
                &ServerEnvelope::Disconnect(Disconnect {
                    reason: reason.clone(),
                }),
            )
            .await?;
            Err(reason)
        }
    }
}

async fn register_device(command: RegisterDeviceCommand) -> Result<(), String> {
    let registry = RegistryDatabase::open(&command.database_url).await?;
    let bundle: DeviceRegistrationBundle =
        serde_json::from_slice(&fs::read(&command.bundle_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    registry.register_device(&bundle, now_unix_ms()).await?;
    println!(
        "registered device {} for member {}",
        bundle.device_id, bundle.member_id
    );
    Ok(())
}

async fn disable_device(command: DisableDeviceCommand) -> Result<(), String> {
    let registry = RegistryDatabase::open(&command.database_url).await?;
    registry.disable_device(&command.device_id).await?;
    println!("disabled device {}", command.device_id);
    Ok(())
}

async fn create_invite(command: CreateInviteCommand) -> Result<(), String> {
    let registry = RegistryDatabase::open(&command.database_url).await?;
    let invite_service = InviteService::new(registry, command.invite_secret.into_bytes());
    let issued_at = now_unix_ms();
    let expires_at = issued_at.saturating_add(command.ttl_seconds.saturating_mul(1000));
    let link = invite_service
        .create_invite(
            command.invite_id,
            command.label,
            command.server_addr,
            command.server_name,
            load_certificate_der(&command.cert_path)?,
            issued_at,
            expires_at,
            command.max_uses,
        )
        .await?;
    println!("{link}");
    Ok(())
}

async fn list_devices(command: DatabaseCommand) -> Result<(), String> {
    let registry = RegistryDatabase::open(&command.database_url).await?;
    let devices = registry.list_devices().await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&devices).map_err(|e| e.to_string())?
    );
    Ok(())
}

async fn list_invites(command: DatabaseCommand) -> Result<(), String> {
    let registry = RegistryDatabase::open(&command.database_url).await?;
    let invites = registry.list_invites().await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&invites).map_err(|e| e.to_string())?
    );
    Ok(())
}

async fn send_server_envelope(
    connection: &TransportConnection,
    envelope: &ServerEnvelope,
) -> Result<(), String> {
    connection
        .send_frame(&TransportFrame::payload(
            bincode::serialize(envelope).map_err(|error| error.to_string())?,
        ))
        .await
        .map_err(|error| error.to_string())
}

async fn receive_client_envelope(
    connection: &TransportConnection,
) -> Result<ClientEnvelope, String> {
    let frame = connection
        .receive_frame()
        .await
        .map_err(|error| error.to_string())?;
    match frame {
        TransportFrame::Payload(bytes) => {
            bincode::deserialize(&bytes).map_err(|error| error.to_string())
        }
        _ => Err("expected payload transport frame".to_string()),
    }
}

fn load_transport_identity(
    server_name: &str,
    cert_path: &PathBuf,
    key_path: &PathBuf,
) -> Result<TransportIdentity, String> {
    let certificate_der = load_certificate_der(cert_path)?;
    let private_key_der = load_private_key_der(key_path)?;
    Ok(TransportIdentity::from_der(
        server_name.to_string(),
        certificate_der,
        private_key_der,
    ))
}

fn load_certificate_der(path: &PathBuf) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| error.to_string())
}

fn load_private_key_der(path: &PathBuf) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| error.to_string())
}

fn parse_flag(args: &[String], flag: &str) -> Result<String, String> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == flag {
            return args
                .get(index + 1)
                .cloned()
                .ok_or_else(|| format!("missing value for {flag}"));
        }
        index += 1;
    }
    Err(format!("missing required flag {flag}"))
}

fn parse_optional_flag(args: &[String], flag: &str) -> Option<String> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == flag {
            return args.get(index + 1).cloned();
        }
        index += 1;
    }
    None
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn print_help() {
    let default_bind = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 7443));
    println!("Usage:");
    println!(
        "  cargo run -p localmessenger_server -- serve --bind {default_bind} --server-name relay.local --cert /path/cert.pem --key /path/key.pem --db /path/server.db --invite-secret changeme --peer-frame-limit 120 --blob-request-limit 32"
    );
    println!(
        "  cargo run -p localmessenger_server -- create-invite --db /path/server.db --invite-secret changeme --label \"Home relay\" --server-addr 203.0.113.10:7443 --server-name relay.local --cert /path/cert.der --ttl-seconds 86400 --max-uses 4"
    );
    println!("  cargo run -p localmessenger_server -- list-invites --db /path/server.db");
    println!(
        "  cargo run -p localmessenger_server -- register-device --db /path/server.db --bundle /path/device-bundle.json"
    );
    println!(
        "  cargo run -p localmessenger_server -- disable-device --db /path/server.db --device-id alice-phone"
    );
    println!("  cargo run -p localmessenger_server -- list-devices --db /path/server.db");
}
