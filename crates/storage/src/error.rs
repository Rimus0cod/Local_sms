use std::error::Error;
use std::fmt;

use localmessenger_core::CoreError;
use localmessenger_crypto::CryptoError;

#[derive(Debug)]
pub enum StorageError {
    Core(CoreError),
    Crypto(CryptoError),
    Sqlite(sqlx::Error),
    Serialization(String),
    InvalidStorageKeyLength(usize),
    InvalidRecordVersion(u8),
    InvalidIdentifier { field: &'static str, value: String },
    EmptyCiphertext,
    LocalDeviceIdentityMismatch,
    LocalPrekeyIdentityMismatch,
    EncryptionFailed,
    DecryptionFailed,
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => write!(formatter, "core error: {error}"),
            Self::Crypto(error) => write!(formatter, "crypto error: {error}"),
            Self::Sqlite(error) => write!(formatter, "sqlite error: {error}"),
            Self::Serialization(error) => write!(formatter, "serialization error: {error}"),
            Self::InvalidStorageKeyLength(length) => {
                write!(formatter, "storage key must be 32 bytes, got {length}")
            }
            Self::InvalidRecordVersion(version) => {
                write!(formatter, "unsupported encrypted record version {version}")
            }
            Self::InvalidIdentifier { field, value } => {
                write!(
                    formatter,
                    "{field} '{value}' contains unsupported characters"
                )
            }
            Self::EmptyCiphertext => formatter.write_str("ciphertext cannot be empty"),
            Self::LocalDeviceIdentityMismatch => {
                formatter.write_str("local device identity does not match the stored identity key")
            }
            Self::LocalPrekeyIdentityMismatch => {
                formatter.write_str("local prekey store identity does not match the device")
            }
            Self::EncryptionFailed => formatter.write_str("at-rest encryption failed"),
            Self::DecryptionFailed => formatter.write_str("at-rest decryption failed"),
        }
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::Crypto(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::Serialization(_)
            | Self::InvalidStorageKeyLength(_)
            | Self::InvalidRecordVersion(_)
            | Self::InvalidIdentifier { .. }
            | Self::EmptyCiphertext
            | Self::LocalDeviceIdentityMismatch
            | Self::LocalPrekeyIdentityMismatch
            | Self::EncryptionFailed
            | Self::DecryptionFailed => None,
        }
    }
}

impl From<CoreError> for StorageError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

impl From<CryptoError> for StorageError {
    fn from(error: CryptoError) -> Self {
        Self::Crypto(error)
    }
}

impl From<sqlx::Error> for StorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<Box<bincode::ErrorKind>> for StorageError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        Self::Serialization(error.to_string())
    }
}
