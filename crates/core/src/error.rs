use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum CoreError {
    EmptyIdentifier(&'static str),
    InvalidIdentifier {
        field: &'static str,
        value: String,
    },
    EmptyDisplayName,
    DuplicateDevice(String),
    ForeignDeviceOwner {
        expected_member_id: String,
        actual_member_id: String,
    },
    MissingDevice(String),
    SafetyNumberMismatch,
    InvalidQrPayloadVersion(u8),
    QrPayloadMismatch,
    Serialization(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(field) => write!(formatter, "{field} cannot be empty"),
            Self::InvalidIdentifier { field, value } => write!(
                formatter,
                "{field} '{value}' must contain only ASCII letters, digits, '-' or '_'"
            ),
            Self::EmptyDisplayName => formatter.write_str("display name cannot be empty"),
            Self::DuplicateDevice(device_id) => {
                write!(formatter, "device '{device_id}' already exists")
            }
            Self::ForeignDeviceOwner {
                expected_member_id,
                actual_member_id,
            } => write!(
                formatter,
                "device owner mismatch: expected '{expected_member_id}', got '{actual_member_id}'"
            ),
            Self::MissingDevice(device_id) => write!(formatter, "device '{device_id}' not found"),
            Self::SafetyNumberMismatch => {
                formatter.write_str("provided safety number did not match")
            }
            Self::InvalidQrPayloadVersion(version) => {
                write!(formatter, "unsupported QR payload version {version}")
            }
            Self::QrPayloadMismatch => formatter.write_str("QR payload does not match this device"),
            Self::Serialization(error) => write!(formatter, "serialization error: {error}"),
        }
    }
}

impl Error for CoreError {}

impl From<Box<bincode::ErrorKind>> for CoreError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        Self::Serialization(error.to_string())
    }
}
