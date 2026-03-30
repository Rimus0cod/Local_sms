use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum CryptoError {
    InvalidSignature,
    MissingSignedPrekey(u32),
    MissingOneTimePrekey(u32),
    MissingSendingChain,
    MissingReceivingChain,
    InvalidHeaderVersion(u8),
    ReplayOrDuplicateMessage(u32),
    MessageNumberTooFarAhead {
        current: u32,
        requested: u32,
        max_skip: u32,
    },
    Serialization(String),
    EncryptionFailed,
    DecryptionFailed,
    InvalidKeyMaterial(&'static str),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSignature => formatter.write_str("invalid prekey signature"),
            Self::MissingSignedPrekey(id) => write!(formatter, "missing signed prekey {id}"),
            Self::MissingOneTimePrekey(id) => write!(formatter, "missing one-time prekey {id}"),
            Self::MissingSendingChain => formatter.write_str("missing sending chain"),
            Self::MissingReceivingChain => formatter.write_str("missing receiving chain"),
            Self::InvalidHeaderVersion(version) => {
                write!(formatter, "unsupported message header version {version}")
            }
            Self::ReplayOrDuplicateMessage(number) => {
                write!(formatter, "message {number} was already processed")
            }
            Self::MessageNumberTooFarAhead {
                current,
                requested,
                max_skip,
            } => write!(
                formatter,
                "message number advanced too far: current={current}, requested={requested}, max_skip={max_skip}"
            ),
            Self::Serialization(error) => write!(formatter, "serialization error: {error}"),
            Self::EncryptionFailed => formatter.write_str("message encryption failed"),
            Self::DecryptionFailed => formatter.write_str("message decryption failed"),
            Self::InvalidKeyMaterial(label) => {
                write!(formatter, "invalid key material for {label}")
            }
        }
    }
}

impl Error for CryptoError {}

impl From<Box<bincode::ErrorKind>> for CryptoError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        Self::Serialization(error.to_string())
    }
}
