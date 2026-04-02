use std::str::FromStr;

use localmessenger_core::{Device, DeviceId};
use localmessenger_discovery::DiscoveredPeer;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Row, Sqlite};

use crate::cipher::AtRestCipher;
use crate::error::StorageError;
use crate::models::{
    StorageKey, StoredLocalDeviceSecrets, StoredMessage, StoredPendingOutbound,
    StoredRemotePeerOffer, validate_identifier,
};

const DEVICE_NAMESPACE: &str = "device";
const LOCAL_SECRETS_NAMESPACE: &str = "local-device-secrets";
const PEER_NAMESPACE: &str = "peer";
const REMOTE_OFFER_NAMESPACE: &str = "remote-peer-offer";
const MESSAGE_NAMESPACE: &str = "message";
const CONVERSATION_NAMESPACE: &str = "conversation";
const PENDING_NAMESPACE: &str = "pending-outbound";

pub struct SqliteStorage {
    pool: Pool<Sqlite>,
    cipher: AtRestCipher,
}

impl SqliteStorage {
    pub async fn open(database_url: &str, storage_key: StorageKey) -> Result<Self, StorageError> {
        let journal_mode = if database_url.contains(":memory:") {
            SqliteJournalMode::Memory
        } else {
            SqliteJournalMode::Wal
        };
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|error| StorageError::Serialization(error.to_string()))?
            .create_if_missing(!database_url.contains(":memory:"))
            .journal_mode(journal_mode)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true);
        let max_connections = if database_url.contains(":memory:") {
            1
        } else {
            4
        };
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await?;

        let storage = Self {
            pool,
            cipher: AtRestCipher::new(storage_key),
        };
        storage.initialize_schema().await?;
        Ok(storage)
    }

    pub async fn upsert_device(&self, device: &Device) -> Result<(), StorageError> {
        let lookup_key = device_lookup_key(device.device_id());
        let blob = self.cipher.encrypt(DEVICE_NAMESPACE, &lookup_key, device)?;
        sqlx::query(
            r#"
            INSERT INTO device_snapshots (lookup_key, encrypted_blob)
            VALUES (?, ?)
            ON CONFLICT(lookup_key) DO UPDATE SET encrypted_blob = excluded.encrypted_blob
            "#,
        )
        .bind(lookup_key.to_vec())
        .bind(blob)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn device(&self, device_id: &DeviceId) -> Result<Option<Device>, StorageError> {
        let lookup_key = device_lookup_key(device_id);
        let row = sqlx::query("SELECT encrypted_blob FROM device_snapshots WHERE lookup_key = ?")
            .bind(lookup_key.to_vec())
            .fetch_optional(&self.pool)
            .await?;

        row.map(|row| {
            let encrypted_blob: Vec<u8> = row.try_get("encrypted_blob")?;
            self.cipher
                .decrypt(DEVICE_NAMESPACE, &lookup_key, &encrypted_blob)
        })
        .transpose()
    }

    pub async fn list_devices(&self) -> Result<Vec<Device>, StorageError> {
        let rows =
            sqlx::query("SELECT lookup_key, encrypted_blob FROM device_snapshots ORDER BY rowid")
                .fetch_all(&self.pool)
                .await?;

        rows.into_iter()
            .map(|row| {
                let lookup_key: Vec<u8> = row.try_get("lookup_key")?;
                let encrypted_blob: Vec<u8> = row.try_get("encrypted_blob")?;
                self.cipher
                    .decrypt(DEVICE_NAMESPACE, &lookup_key, &encrypted_blob)
            })
            .collect()
    }

    pub async fn store_local_device_secrets(
        &self,
        secrets: &StoredLocalDeviceSecrets,
    ) -> Result<(), StorageError> {
        self.upsert_device(&secrets.device).await?;

        let lookup_key = local_secrets_lookup_key(secrets.device.device_id());
        let blob = self
            .cipher
            .encrypt(LOCAL_SECRETS_NAMESPACE, &lookup_key, secrets)?;
        sqlx::query(
            r#"
            INSERT INTO local_device_secrets (lookup_key, encrypted_blob)
            VALUES (?, ?)
            ON CONFLICT(lookup_key) DO UPDATE SET encrypted_blob = excluded.encrypted_blob
            "#,
        )
        .bind(lookup_key.to_vec())
        .bind(blob)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn local_device_secrets(
        &self,
        device_id: &DeviceId,
    ) -> Result<Option<StoredLocalDeviceSecrets>, StorageError> {
        let lookup_key = local_secrets_lookup_key(device_id);
        let row =
            sqlx::query("SELECT encrypted_blob FROM local_device_secrets WHERE lookup_key = ?")
                .bind(lookup_key.to_vec())
                .fetch_optional(&self.pool)
                .await?;

        row.map(|row| {
            let encrypted_blob: Vec<u8> = row.try_get("encrypted_blob")?;
            self.cipher
                .decrypt(LOCAL_SECRETS_NAMESPACE, &lookup_key, &encrypted_blob)
        })
        .transpose()
    }

    pub async fn upsert_peer(&self, peer: &DiscoveredPeer) -> Result<(), StorageError> {
        let lookup_key = peer_lookup_key(peer.device_id.as_str());
        let blob = self.cipher.encrypt(PEER_NAMESPACE, &lookup_key, peer)?;
        sqlx::query(
            r#"
            INSERT INTO peer_snapshots (lookup_key, encrypted_blob)
            VALUES (?, ?)
            ON CONFLICT(lookup_key) DO UPDATE SET encrypted_blob = excluded.encrypted_blob
            "#,
        )
        .bind(lookup_key.to_vec())
        .bind(blob)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_peers(&self) -> Result<Vec<DiscoveredPeer>, StorageError> {
        let rows =
            sqlx::query("SELECT lookup_key, encrypted_blob FROM peer_snapshots ORDER BY rowid")
                .fetch_all(&self.pool)
                .await?;

        rows.into_iter()
            .map(|row| {
                let lookup_key: Vec<u8> = row.try_get("lookup_key")?;
                let encrypted_blob: Vec<u8> = row.try_get("encrypted_blob")?;
                self.cipher
                    .decrypt(PEER_NAMESPACE, &lookup_key, &encrypted_blob)
            })
            .collect()
    }

    pub async fn upsert_remote_peer_offer(
        &self,
        offer: &StoredRemotePeerOffer,
    ) -> Result<(), StorageError> {
        let lookup_key = opaque_lookup_key(REMOTE_OFFER_NAMESPACE, &offer.invite.device_id);
        let blob = self
            .cipher
            .encrypt(REMOTE_OFFER_NAMESPACE, &lookup_key, offer)?;
        sqlx::query(
            r#"
            INSERT INTO remote_peer_offers (lookup_key, encrypted_blob)
            VALUES (?, ?)
            ON CONFLICT(lookup_key) DO UPDATE SET encrypted_blob = excluded.encrypted_blob
            "#,
        )
        .bind(lookup_key.to_vec())
        .bind(blob)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_remote_peer_offers(&self) -> Result<Vec<StoredRemotePeerOffer>, StorageError> {
        let rows =
            sqlx::query("SELECT lookup_key, encrypted_blob FROM remote_peer_offers ORDER BY rowid")
                .fetch_all(&self.pool)
                .await?;

        rows.into_iter()
            .map(|row| {
                let lookup_key: Vec<u8> = row.try_get("lookup_key")?;
                let encrypted_blob: Vec<u8> = row.try_get("encrypted_blob")?;
                self.cipher
                    .decrypt(REMOTE_OFFER_NAMESPACE, &lookup_key, &encrypted_blob)
            })
            .collect()
    }

    pub async fn append_message(&self, message: &StoredMessage) -> Result<(), StorageError> {
        validate_identifier("message_id", &message.message_id)?;
        validate_identifier("conversation_id", &message.conversation_id)?;

        let message_key = opaque_lookup_key(MESSAGE_NAMESPACE, &message.message_id);
        let conversation_key = opaque_lookup_key(CONVERSATION_NAMESPACE, &message.conversation_id);
        let blob = self
            .cipher
            .encrypt(MESSAGE_NAMESPACE, &message_key, message)?;

        sqlx::query(
            r#"
            INSERT INTO message_log (message_key, conversation_key, encrypted_blob)
            VALUES (?, ?, ?)
            ON CONFLICT(message_key) DO UPDATE SET
                conversation_key = excluded.conversation_key,
                encrypted_blob = excluded.encrypted_blob
            "#,
        )
        .bind(message_key.to_vec())
        .bind(conversation_key.to_vec())
        .bind(blob)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn messages_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<StoredMessage>, StorageError> {
        validate_identifier("conversation_id", conversation_id)?;

        let conversation_key = opaque_lookup_key(CONVERSATION_NAMESPACE, conversation_id);
        let rows = sqlx::query(
            r#"
            SELECT message_key, encrypted_blob
            FROM message_log
            WHERE conversation_key = ?
            ORDER BY ordinal
            "#,
        )
        .bind(conversation_key.to_vec())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let message_key: Vec<u8> = row.try_get("message_key")?;
                let encrypted_blob: Vec<u8> = row.try_get("encrypted_blob")?;
                self.cipher
                    .decrypt(MESSAGE_NAMESPACE, &message_key, &encrypted_blob)
            })
            .collect()
    }

    pub async fn upsert_pending_outbound(
        &self,
        entry: &StoredPendingOutbound,
    ) -> Result<(), StorageError> {
        let peer_key = opaque_lookup_key(PENDING_NAMESPACE, &entry.peer_device_id);
        let message_key = opaque_lookup_key(PENDING_NAMESPACE, &entry.message_id);
        let blob = self
            .cipher
            .encrypt(PENDING_NAMESPACE, &message_key, entry)?;
        sqlx::query(
            r#"
            INSERT INTO pending_outbound_queue (peer_key, message_key, delivery_order, encrypted_blob)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(message_key) DO UPDATE SET
                delivery_order = excluded.delivery_order,
                encrypted_blob = excluded.encrypted_blob
            "#,
        )
        .bind(peer_key.to_vec())
        .bind(message_key.to_vec())
        .bind(entry.delivery_order as i64)
        .bind(blob)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn pending_outbound_for_peer(
        &self,
        peer_device_id: &str,
    ) -> Result<Vec<StoredPendingOutbound>, StorageError> {
        validate_identifier("peer_device_id", peer_device_id)?;
        let peer_key = opaque_lookup_key(PENDING_NAMESPACE, peer_device_id);
        let rows = sqlx::query(
            r#"
            SELECT message_key, encrypted_blob
            FROM pending_outbound_queue
            WHERE peer_key = ?
            ORDER BY delivery_order
            "#,
        )
        .bind(peer_key.to_vec())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let message_key: Vec<u8> = row.try_get("message_key")?;
                let encrypted_blob: Vec<u8> = row.try_get("encrypted_blob")?;
                let key: [u8; 32] = message_key
                    .try_into()
                    .map_err(|_| StorageError::DecryptionFailed)?;
                self.cipher
                    .decrypt(PENDING_NAMESPACE, &key, &encrypted_blob)
            })
            .collect()
    }

    pub async fn remove_pending_outbound(
        &self,
        peer_device_id: &str,
        message_id: &str,
    ) -> Result<(), StorageError> {
        validate_identifier("peer_device_id", peer_device_id)?;
        validate_identifier("message_id", message_id)?;
        let message_key = opaque_lookup_key(PENDING_NAMESPACE, message_id);
        sqlx::query("DELETE FROM pending_outbound_queue WHERE message_key = ?")
            .bind(message_key.to_vec())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn clear_pending_outbound_for_peer(
        &self,
        peer_device_id: &str,
    ) -> Result<(), StorageError> {
        validate_identifier("peer_device_id", peer_device_id)?;
        let peer_key = opaque_lookup_key(PENDING_NAMESPACE, peer_device_id);
        sqlx::query("DELETE FROM pending_outbound_queue WHERE peer_key = ?")
            .bind(peer_key.to_vec())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) async fn raw_encrypted_blobs(
        &self,
        table: &'static str,
    ) -> Result<Vec<Vec<u8>>, StorageError> {
        let query = format!("SELECT encrypted_blob FROM {table} ORDER BY rowid");
        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|row| row.try_get("encrypted_blob").map_err(StorageError::from))
            .collect()
    }

    async fn initialize_schema(&self) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS device_snapshots (
                lookup_key BLOB PRIMARY KEY,
                encrypted_blob BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS local_device_secrets (
                lookup_key BLOB PRIMARY KEY,
                encrypted_blob BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS peer_snapshots (
                lookup_key BLOB PRIMARY KEY,
                encrypted_blob BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS remote_peer_offers (
                lookup_key BLOB PRIMARY KEY,
                encrypted_blob BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS message_log (
                ordinal INTEGER PRIMARY KEY AUTOINCREMENT,
                message_key BLOB NOT NULL UNIQUE,
                conversation_key BLOB NOT NULL,
                encrypted_blob BLOB NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_message_log_conversation
            ON message_log (conversation_key, ordinal);

            CREATE TABLE IF NOT EXISTS pending_outbound_queue (
                peer_key BLOB NOT NULL,
                message_key BLOB NOT NULL UNIQUE,
                delivery_order INTEGER NOT NULL,
                encrypted_blob BLOB NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_pending_outbound_peer
            ON pending_outbound_queue (peer_key, delivery_order);
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn device_lookup_key(device_id: &DeviceId) -> [u8; 32] {
    opaque_lookup_key(DEVICE_NAMESPACE, device_id.as_str())
}

fn local_secrets_lookup_key(device_id: &DeviceId) -> [u8; 32] {
    opaque_lookup_key(LOCAL_SECRETS_NAMESPACE, device_id.as_str())
}

fn peer_lookup_key(device_id: &str) -> [u8; 32] {
    opaque_lookup_key(PEER_NAMESPACE, device_id)
}

fn opaque_lookup_key(namespace: &'static str, identifier: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"localmessenger/storage/index/v1");
    hasher.update(namespace.as_bytes());
    hasher.update(identifier.as_bytes());
    hasher.finalize().into()
}
