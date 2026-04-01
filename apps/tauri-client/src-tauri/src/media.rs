use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use localmessenger_server_protocol::{MAX_BLOB_CHUNK_BYTES, MAX_RELAY_BLOB_BYTES, MediaKind};
use localmessenger_transport::{
    ReconnectPolicy, TransportEndpoint, TransportEndpointConfig, TransportFrame, TransportIdentity,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const RELAY_MEDIA_MAX_BYTES: usize = MAX_RELAY_BLOB_BYTES as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRoute {
    RelayBlobStore,
    DirectQuic,
}

impl MediaRoute {
    pub fn label(self) -> &'static str {
        match self {
            Self::RelayBlobStore => "server_blob_store",
            Self::DirectQuic => "p2p_quic_direct",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedBlob {
    pub ciphertext: Vec<u8>,
    pub key: [u8; 32],
    pub nonce: [u8; 12],
    pub sha256_hex: String,
}

#[derive(Debug, Clone)]
pub struct DirectTransferReceipt {
    pub transfer_id: String,
    pub transferred_bytes: u64,
    pub sha256_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DirectTransferEnvelope {
    Meta {
        transfer_id: String,
        file_name: String,
        mime_type: String,
        sha256_hex: String,
        total_bytes: u64,
    },
    Chunk {
        offset: u64,
        bytes: Vec<u8>,
        is_last: bool,
    },
}

pub fn media_kind_for_mime(mime_type: &str) -> MediaKind {
    if mime_type.starts_with("image/") {
        MediaKind::Photo
    } else {
        MediaKind::File
    }
}

pub fn encrypt_blob(plaintext: &[u8]) -> Result<EncryptedBlob, String> {
    let mut key = [0_u8; 32];
    let mut nonce = [0_u8; 12];
    OsRng.fill_bytes(&mut key);
    OsRng.fill_bytes(&mut nonce);

    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|error| error.to_string())?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| "blob encryption failed".to_string())?;

    Ok(EncryptedBlob {
        sha256_hex: sha256_hex(&ciphertext),
        ciphertext,
        key,
        nonce,
    })
}

pub fn decrypt_blob(blob: &EncryptedBlob) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(&blob.key).map_err(|error| error.to_string())?;
    cipher
        .decrypt(Nonce::from_slice(&blob.nonce), blob.ciphertext.as_ref())
        .map_err(|_| "blob decryption failed".to_string())
}

pub fn data_url(mime_type: &str, bytes: &[u8]) -> String {
    format!(
        "data:{mime_type};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

pub async fn transfer_blob_over_quic(
    file_name: &str,
    mime_type: &str,
    ciphertext: Vec<u8>,
    digest_hex: String,
) -> Result<DirectTransferReceipt, String> {
    let transfer_id = format!("direct-{}", random_hex(8));
    let server_config =
        TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
    let server_identity = TransportIdentity::generate(server_config.server_name.clone())
        .map_err(|error| error.to_string())?;
    let server = TransportEndpoint::bind(server_config, server_identity.clone())
        .map_err(|error| error.to_string())?;
    let server_addr = server.local_addr().map_err(|error| error.to_string())?;

    let expected_sha = digest_hex.clone();
    let expected_transfer_id = transfer_id.clone();
    let file_name_owned = file_name.to_string();
    let mime_type_owned = mime_type.to_string();
    let accept_task = tokio::spawn(async move {
        let connection = server.accept().await.map_err(|error| error.to_string())?;
        let meta = read_direct_envelope(&connection).await?;
        match meta {
            DirectTransferEnvelope::Meta {
                transfer_id,
                file_name,
                mime_type,
                sha256_hex: claimed_sha256_hex,
                total_bytes,
            } => {
                if transfer_id != expected_transfer_id {
                    return Err("direct transfer id mismatch".to_string());
                }
                if file_name != file_name_owned || mime_type != mime_type_owned {
                    return Err("direct transfer metadata mismatch".to_string());
                }
                let mut received = Vec::with_capacity(total_bytes as usize);
                loop {
                    match read_direct_envelope(&connection).await? {
                        DirectTransferEnvelope::Chunk {
                            offset,
                            bytes,
                            is_last,
                        } => {
                            if offset != received.len() as u64 {
                                return Err("direct transfer chunk order mismatch".to_string());
                            }
                            received.extend_from_slice(&bytes);
                            if is_last {
                                break;
                            }
                        }
                        DirectTransferEnvelope::Meta { .. } => {
                            return Err("unexpected metadata envelope during transfer".to_string());
                        }
                    }
                }
                if received.len() as u64 != total_bytes {
                    return Err("direct transfer length mismatch".to_string());
                }
                if claimed_sha256_hex != expected_sha || claimed_sha256_hex != sha256_hex(&received)
                {
                    return Err("direct transfer digest mismatch".to_string());
                }
                Ok::<(), String>(())
            }
            DirectTransferEnvelope::Chunk { .. } => {
                Err("expected metadata frame first".to_string())
            }
        }
    });

    let client_config =
        TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
    let client_identity = TransportIdentity::generate(client_config.server_name.clone())
        .map_err(|error| error.to_string())?;
    let client = TransportEndpoint::bind(client_config, client_identity)
        .map_err(|error| error.to_string())?;
    let connection = client
        .connect(
            server_addr,
            &server_identity.certificate_der,
            &ReconnectPolicy::new(3, Duration::from_millis(40), Duration::from_millis(180)),
        )
        .await
        .map_err(|error| error.to_string())?;

    write_direct_envelope(
        &connection,
        &DirectTransferEnvelope::Meta {
            transfer_id: transfer_id.clone(),
            file_name: file_name.to_string(),
            mime_type: mime_type.to_string(),
            sha256_hex: digest_hex.clone(),
            total_bytes: ciphertext.len() as u64,
        },
    )
    .await?;
    for (index, chunk) in ciphertext.chunks(MAX_BLOB_CHUNK_BYTES).enumerate() {
        let offset = index.saturating_mul(MAX_BLOB_CHUNK_BYTES) as u64;
        let is_last = offset + chunk.len() as u64 >= ciphertext.len() as u64;
        write_direct_envelope(
            &connection,
            &DirectTransferEnvelope::Chunk {
                offset,
                bytes: chunk.to_vec(),
                is_last,
            },
        )
        .await?;
    }

    accept_task.await.map_err(|error| error.to_string())??;

    Ok(DirectTransferReceipt {
        transfer_id,
        transferred_bytes: ciphertext.len() as u64,
        sha256_hex: digest_hex,
    })
}

async fn write_direct_envelope(
    connection: &localmessenger_transport::TransportConnection,
    envelope: &DirectTransferEnvelope,
) -> Result<(), String> {
    connection
        .send_frame(&TransportFrame::payload(
            bincode::serialize(envelope).map_err(|error| error.to_string())?,
        ))
        .await
        .map_err(|error| error.to_string())
}

async fn read_direct_envelope(
    connection: &localmessenger_transport::TransportConnection,
) -> Result<DirectTransferEnvelope, String> {
    match connection
        .receive_frame()
        .await
        .map_err(|error| error.to_string())?
    {
        TransportFrame::Payload(bytes) => {
            bincode::deserialize(&bytes).map_err(|error| error.to_string())
        }
        _ => Err("expected payload transport frame".to_string()),
    }
}

fn random_hex(bytes: usize) -> String {
    let mut data = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut data);
    let mut output = String::with_capacity(bytes * 2);
    for byte in data {
        output.push(nibble_to_hex(byte >> 4));
        output.push(nibble_to_hex(byte & 0x0f));
    }
    output
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
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

#[cfg(test)]
mod tests {
    use super::{decrypt_blob, encrypt_blob, transfer_blob_over_quic};

    #[test]
    fn blob_round_trip_encrypts_and_decrypts() {
        let encrypted = encrypt_blob(b"hello photo").expect("encrypt");
        let decrypted = decrypt_blob(&encrypted).expect("decrypt");
        assert_eq!(decrypted, b"hello photo");
        assert_eq!(encrypted.sha256_hex.len(), 64);
    }

    #[tokio::test]
    #[ignore = "network-required: binds UDP sockets"]
    async fn direct_transfer_moves_ciphertext_over_quic() {
        let payload = vec![9_u8; 180_000];
        let receipt = transfer_blob_over_quic(
            "large.mov",
            "video/quicktime",
            payload.clone(),
            super::sha256_hex(&payload),
        )
        .await
        .expect("transfer");
        assert_eq!(receipt.transferred_bytes, 180_000);
    }
}
