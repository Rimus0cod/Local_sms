use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Row, Sqlite};
use std::path::Path;
use std::str::FromStr;

use crate::auth::RegisteredDevice;
use localmessenger_server_protocol::{
    DeviceRegistrationBundle, InviteClaims, MediaKind, StoredBlob,
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct RegistryDatabase {
    pool: Pool<Sqlite>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListedDevice {
    pub member_id: String,
    pub device_id: String,
    pub device_name: String,
    pub status: String,
    pub created_at_unix_ms: i64,
    pub last_seen_at_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListedInvite {
    pub invite_id: String,
    pub label: String,
    pub expires_at_unix_ms: i64,
    pub max_uses: u32,
    pub used_count: u32,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct StoredInvite {
    pub invite_id: String,
    pub label: String,
    pub expires_at_unix_ms: i64,
    pub max_uses: u32,
    pub used_count: u32,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct QueuedPeerFrameRecord {
    pub row_id: i64,
    pub sender_device_id: String,
    pub recipient_device_id: String,
    pub payload: Vec<u8>,
    pub queued_at_unix_ms: i64,
}

#[derive(Debug, Clone)]
pub struct StoredBlobRecord {
    pub metadata: StoredBlob,
    pub ciphertext: Vec<u8>,
}

impl RegistryDatabase {
    pub async fn open(database_url: &str) -> Result<Self, String> {
        let journal_mode = if database_url.contains(":memory:") {
            SqliteJournalMode::Memory
        } else {
            SqliteJournalMode::Wal
        };
        let options = if database_url == ":memory:" {
            SqliteConnectOptions::from_str("sqlite::memory:")
        } else if database_url.starts_with("sqlite:") {
            SqliteConnectOptions::from_str(database_url)
        } else {
            let path = Path::new(database_url);
            let sqlite_url = format!("sqlite://{}", path.display());
            SqliteConnectOptions::from_str(&sqlite_url)
        }
        .map_err(|error| error.to_string())?
        .create_if_missing(!database_url.contains(":memory:"))
        .journal_mode(journal_mode)
        .synchronous(SqliteSynchronous::Full)
        .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(if database_url.contains(":memory:") {
                1
            } else {
                4
            })
            .connect_with(options)
            .await
            .map_err(|error| error.to_string())?;

        let db = Self { pool };
        db.initialize().await?;
        Ok(db)
    }

    pub async fn register_device(
        &self,
        bundle: &DeviceRegistrationBundle,
        now_unix_ms: i64,
    ) -> Result<(), String> {
        bundle.validate()?;
        sqlx::query(
            r#"
            INSERT INTO registered_devices (
                device_id,
                member_id,
                device_name,
                auth_public_key,
                status,
                created_at_unix_ms,
                last_seen_at_unix_ms
            )
            VALUES (?, ?, ?, ?, 'active', ?, NULL)
            ON CONFLICT(device_id) DO UPDATE SET
                member_id = excluded.member_id,
                device_name = excluded.device_name,
                auth_public_key = excluded.auth_public_key,
                status = 'active'
            "#,
        )
        .bind(&bundle.device_id)
        .bind(&bundle.member_id)
        .bind(&bundle.device_name)
        .bind(bundle.auth_public_key.to_vec())
        .bind(now_unix_ms)
        .execute(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn disable_device(&self, device_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE registered_devices SET status = 'disabled' WHERE device_id = ?")
            .bind(device_id)
            .execute(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn touch_last_seen(&self, device_id: &str, now_unix_ms: i64) -> Result<(), String> {
        sqlx::query("UPDATE registered_devices SET last_seen_at_unix_ms = ? WHERE device_id = ?")
            .bind(now_unix_ms)
            .bind(device_id)
            .execute(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn registered_device(
        &self,
        device_id: &str,
    ) -> Result<Option<RegisteredDevice>, String> {
        let row = sqlx::query(
            r#"
            SELECT member_id, device_id, device_name, auth_public_key, status
            FROM registered_devices
            WHERE device_id = ?
            "#,
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| error.to_string())?;

        row.map(|row| {
            let auth_public_key: Vec<u8> =
                row.try_get("auth_public_key").map_err(|e| e.to_string())?;
            let auth_public_key: [u8; 32] = auth_public_key
                .try_into()
                .map_err(|_| "stored auth public key must be 32 bytes".to_string())?;
            Ok(RegisteredDevice {
                member_id: row.try_get("member_id").map_err(|e| e.to_string())?,
                device_id: row.try_get("device_id").map_err(|e| e.to_string())?,
                device_name: row.try_get("device_name").map_err(|e| e.to_string())?,
                auth_public_key,
                disabled: row
                    .try_get::<String, _>("status")
                    .map_err(|e| e.to_string())?
                    != "active",
            })
        })
        .transpose()
    }

    pub async fn list_devices(&self) -> Result<Vec<ListedDevice>, String> {
        let rows = sqlx::query(
            r#"
            SELECT member_id, device_id, device_name, status, created_at_unix_ms, last_seen_at_unix_ms
            FROM registered_devices
            ORDER BY member_id, device_id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;

        rows.into_iter()
            .map(|row| {
                Ok(ListedDevice {
                    member_id: row.try_get("member_id").map_err(|e| e.to_string())?,
                    device_id: row.try_get("device_id").map_err(|e| e.to_string())?,
                    device_name: row.try_get("device_name").map_err(|e| e.to_string())?,
                    status: row.try_get("status").map_err(|e| e.to_string())?,
                    created_at_unix_ms: row
                        .try_get("created_at_unix_ms")
                        .map_err(|e| e.to_string())?,
                    last_seen_at_unix_ms: row
                        .try_get("last_seen_at_unix_ms")
                        .map_err(|e| e.to_string())?,
                })
            })
            .collect()
    }

    pub async fn create_invite(
        &self,
        claims: &InviteClaims,
        now_unix_ms: i64,
    ) -> Result<(), String> {
        claims.validate()?;
        sqlx::query(
            r#"
            INSERT INTO invite_tokens (
                invite_id,
                label,
                expires_at_unix_ms,
                max_uses,
                used_count,
                status,
                created_at_unix_ms
            )
            VALUES (?, ?, ?, ?, 0, 'active', ?)
            ON CONFLICT(invite_id) DO UPDATE SET
                label = excluded.label,
                expires_at_unix_ms = excluded.expires_at_unix_ms,
                max_uses = excluded.max_uses,
                status = 'active'
            "#,
        )
        .bind(&claims.invite_id)
        .bind(&claims.label)
        .bind(claims.expires_at_unix_ms)
        .bind(claims.max_uses as i64)
        .bind(now_unix_ms)
        .execute(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn invite(&self, invite_id: &str) -> Result<Option<StoredInvite>, String> {
        let row = sqlx::query(
            r#"
            SELECT invite_id, label, expires_at_unix_ms, max_uses, used_count, status
            FROM invite_tokens
            WHERE invite_id = ?
            "#,
        )
        .bind(invite_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| error.to_string())?;

        row.map(|row| {
            Ok(StoredInvite {
                invite_id: row.try_get("invite_id").map_err(|e| e.to_string())?,
                label: row.try_get("label").map_err(|e| e.to_string())?,
                expires_at_unix_ms: row
                    .try_get("expires_at_unix_ms")
                    .map_err(|e| e.to_string())?,
                max_uses: row
                    .try_get::<i64, _>("max_uses")
                    .map_err(|e| e.to_string())? as u32,
                used_count: row
                    .try_get::<i64, _>("used_count")
                    .map_err(|e| e.to_string())? as u32,
                status: row.try_get("status").map_err(|e| e.to_string())?,
            })
        })
        .transpose()
    }

    pub async fn mark_invite_used(&self, invite_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE invite_tokens SET used_count = used_count + 1 WHERE invite_id = ?")
            .bind(invite_id)
            .execute(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn list_invites(&self) -> Result<Vec<ListedInvite>, String> {
        let rows = sqlx::query(
            r#"
            SELECT invite_id, label, expires_at_unix_ms, max_uses, used_count, status
            FROM invite_tokens
            ORDER BY rowid DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;

        rows.into_iter()
            .map(|row| {
                Ok(ListedInvite {
                    invite_id: row.try_get("invite_id").map_err(|e| e.to_string())?,
                    label: row.try_get("label").map_err(|e| e.to_string())?,
                    expires_at_unix_ms: row
                        .try_get("expires_at_unix_ms")
                        .map_err(|e| e.to_string())?,
                    max_uses: row
                        .try_get::<i64, _>("max_uses")
                        .map_err(|e| e.to_string())? as u32,
                    used_count: row
                        .try_get::<i64, _>("used_count")
                        .map_err(|e| e.to_string())? as u32,
                    status: row.try_get("status").map_err(|e| e.to_string())?,
                })
            })
            .collect()
    }

    pub async fn queue_peer_frame(
        &self,
        sender_device_id: &str,
        recipient_device_id: &str,
        payload: &[u8],
        queued_at_unix_ms: i64,
    ) -> Result<i64, String> {
        let result = sqlx::query(
            r#"
            INSERT INTO queued_peer_frames (
                sender_device_id,
                recipient_device_id,
                payload,
                queued_at_unix_ms
            )
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(sender_device_id)
        .bind(recipient_device_id)
        .bind(payload)
        .bind(queued_at_unix_ms)
        .execute(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(result.last_insert_rowid())
    }

    pub async fn queued_peer_frames_for_recipient(
        &self,
        recipient_device_id: &str,
    ) -> Result<Vec<QueuedPeerFrameRecord>, String> {
        let rows = sqlx::query(
            r#"
            SELECT rowid, sender_device_id, recipient_device_id, payload, queued_at_unix_ms
            FROM queued_peer_frames
            WHERE recipient_device_id = ?
            ORDER BY rowid
            "#,
        )
        .bind(recipient_device_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;

        rows.into_iter()
            .map(|row| {
                Ok(QueuedPeerFrameRecord {
                    row_id: row.try_get("rowid").map_err(|e| e.to_string())?,
                    sender_device_id: row.try_get("sender_device_id").map_err(|e| e.to_string())?,
                    recipient_device_id: row
                        .try_get("recipient_device_id")
                        .map_err(|e| e.to_string())?,
                    payload: row.try_get("payload").map_err(|e| e.to_string())?,
                    queued_at_unix_ms: row
                        .try_get("queued_at_unix_ms")
                        .map_err(|e| e.to_string())?,
                })
            })
            .collect()
    }

    pub async fn delete_queued_peer_frame(&self, row_id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM queued_peer_frames WHERE rowid = ?")
            .bind(row_id)
            .execute(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn store_blob(&self, blob: &StoredBlob, ciphertext: &[u8]) -> Result<(), String> {
        sqlx::query(
            r#"
            INSERT INTO encrypted_blobs (
                blob_id,
                uploaded_by_device_id,
                file_name,
                mime_type,
                media_kind,
                plaintext_bytes,
                ciphertext_bytes,
                sha256_hex,
                ciphertext,
                created_at_unix_ms
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&blob.blob_id)
        .bind(&blob.uploaded_by_device_id)
        .bind(&blob.file_name)
        .bind(&blob.mime_type)
        .bind(media_kind_code(blob.media_kind))
        .bind(blob.plaintext_bytes as i64)
        .bind(blob.ciphertext_bytes as i64)
        .bind(&blob.sha256_hex)
        .bind(ciphertext)
        .bind(blob.created_at_unix_ms)
        .execute(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn blob(&self, blob_id: &str) -> Result<Option<StoredBlobRecord>, String> {
        let row = sqlx::query(
            r#"
            SELECT
                blob_id,
                uploaded_by_device_id,
                file_name,
                mime_type,
                media_kind,
                plaintext_bytes,
                ciphertext_bytes,
                sha256_hex,
                ciphertext,
                created_at_unix_ms
            FROM encrypted_blobs
            WHERE blob_id = ?
            "#,
        )
        .bind(blob_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| error.to_string())?;

        row.map(|row| {
            Ok(StoredBlobRecord {
                metadata: StoredBlob {
                    blob_id: row.try_get("blob_id").map_err(|e| e.to_string())?,
                    uploaded_by_device_id: row
                        .try_get("uploaded_by_device_id")
                        .map_err(|e| e.to_string())?,
                    file_name: row.try_get("file_name").map_err(|e| e.to_string())?,
                    mime_type: row.try_get("mime_type").map_err(|e| e.to_string())?,
                    media_kind: parse_media_kind(
                        &row.try_get::<String, _>("media_kind")
                            .map_err(|e| e.to_string())?,
                    )?,
                    plaintext_bytes: row
                        .try_get::<i64, _>("plaintext_bytes")
                        .map_err(|e| e.to_string())? as u64,
                    ciphertext_bytes: row
                        .try_get::<i64, _>("ciphertext_bytes")
                        .map_err(|e| e.to_string())? as u64,
                    sha256_hex: row.try_get("sha256_hex").map_err(|e| e.to_string())?,
                    created_at_unix_ms: row
                        .try_get("created_at_unix_ms")
                        .map_err(|e| e.to_string())?,
                },
                ciphertext: row.try_get("ciphertext").map_err(|e| e.to_string())?,
            })
        })
        .transpose()
    }

    async fn initialize(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS registered_devices (
                device_id TEXT PRIMARY KEY,
                member_id TEXT NOT NULL,
                device_name TEXT NOT NULL,
                auth_public_key BLOB NOT NULL,
                status TEXT NOT NULL,
                created_at_unix_ms INTEGER NOT NULL,
                last_seen_at_unix_ms INTEGER NULL
            );

            CREATE TABLE IF NOT EXISTS invite_tokens (
                invite_id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                expires_at_unix_ms INTEGER NOT NULL,
                max_uses INTEGER NOT NULL,
                used_count INTEGER NOT NULL,
                status TEXT NOT NULL,
                created_at_unix_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS queued_peer_frames (
                sender_device_id TEXT NOT NULL,
                recipient_device_id TEXT NOT NULL,
                payload BLOB NOT NULL,
                queued_at_unix_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS encrypted_blobs (
                blob_id TEXT PRIMARY KEY,
                uploaded_by_device_id TEXT NOT NULL,
                file_name TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                media_kind TEXT NOT NULL,
                plaintext_bytes INTEGER NOT NULL,
                ciphertext_bytes INTEGER NOT NULL,
                sha256_hex TEXT NOT NULL,
                ciphertext BLOB NOT NULL,
                created_at_unix_ms INTEGER NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn media_kind_code(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Photo => "photo",
        MediaKind::File => "file",
    }
}

fn parse_media_kind(value: &str) -> Result<MediaKind, String> {
    match value {
        "photo" => Ok(MediaKind::Photo),
        "file" => Ok(MediaKind::File),
        _ => Err(format!("unsupported media kind '{value}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::RegistryDatabase;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use localmessenger_core::{DeviceId, MemberId};
    use localmessenger_server_protocol::{
        DeviceRegistrationBundle, InviteClaims, MediaKind, SERVER_PROTOCOL_VERSION, StoredBlob,
    };

    #[tokio::test]
    async fn register_list_and_disable_device() {
        let db = RegistryDatabase::open(":memory:").await.expect("db");
        let bundle = DeviceRegistrationBundle::new(
            &MemberId::new("alice").expect("member"),
            &DeviceId::new("alice-phone").expect("device"),
            "Alice Phone",
            [9_u8; 32],
        );

        db.register_device(&bundle, 123).await.expect("register");
        let devices = db.list_devices().await.expect("list");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].status, "active");

        db.disable_device("alice-phone").await.expect("disable");
        let stored = db
            .registered_device("alice-phone")
            .await
            .expect("load")
            .expect("device");
        assert!(stored.disabled);
    }

    #[tokio::test]
    async fn invite_and_queue_records_are_persisted() {
        let db = RegistryDatabase::open(":memory:").await.expect("db");
        let invite = InviteClaims {
            version: SERVER_PROTOCOL_VERSION,
            invite_id: "inv-1".to_string(),
            label: "Kitchen relay".to_string(),
            server_addr: "127.0.0.1:7443".to_string(),
            server_name: "relay.local".to_string(),
            server_certificate_der_base64: URL_SAFE_NO_PAD.encode([7_u8; 8]),
            issued_at_unix_ms: 10,
            expires_at_unix_ms: 100,
            max_uses: 2,
        };

        db.create_invite(&invite, 11).await.expect("create invite");
        let stored = db.invite("inv-1").await.expect("load").expect("invite");
        assert_eq!(stored.invite_id, "inv-1");
        assert_eq!(stored.max_uses, 2);

        db.mark_invite_used("inv-1").await.expect("mark used");
        let listed = db.list_invites().await.expect("list");
        assert_eq!(listed[0].used_count, 1);

        let row_id = db
            .queue_peer_frame("alice-phone", "bob-phone", b"ciphertext", 55)
            .await
            .expect("queue");
        let queued = db
            .queued_peer_frames_for_recipient("bob-phone")
            .await
            .expect("queued");
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].sender_device_id, "alice-phone");
        assert_eq!(queued[0].payload, b"ciphertext");

        db.delete_queued_peer_frame(row_id).await.expect("delete");
        assert!(
            db.queued_peer_frames_for_recipient("bob-phone")
                .await
                .expect("queued after delete")
                .is_empty()
        );

        let blob = StoredBlob {
            blob_id: "blob-1".to_string(),
            uploaded_by_device_id: "alice-phone".to_string(),
            file_name: "photo.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            media_kind: MediaKind::Photo,
            plaintext_bytes: 128,
            ciphertext_bytes: 144,
            sha256_hex: "b".repeat(64),
            created_at_unix_ms: 77,
        };
        db.store_blob(&blob, b"cipherblob")
            .await
            .expect("store blob");
        let stored_blob = db.blob("blob-1").await.expect("load blob").expect("blob");
        assert_eq!(stored_blob.metadata.file_name, "photo.jpg");
        assert_eq!(stored_blob.ciphertext, b"cipherblob");
    }
}
