use std::error::Error;
use std::fmt;

use localmessenger_core::CoreError;
use localmessenger_crypto::CryptoError;
use localmessenger_transport::TransportError;

#[derive(Debug)]
pub enum MessagingError {
    Core(CoreError),
    Crypto(CryptoError),
    Transport(TransportError),
    Serialization(String),
    InvalidHandshakeVersion(u8),
    InvalidEnvelopeVersion(u8),
    InvalidSenderKeyDistributionVersion(u8),
    InvalidGroupMessageVersion(u8),
    UnexpectedFrame(&'static str),
    LocalDeviceIdentityMismatch,
    RemoteOfferMismatch(&'static str),
    RemoteBindingMismatch(&'static str),
    TransportBindingMismatch,
    SessionPeerMismatch,
    InvalidIdentifier {
        field: &'static str,
        value: String,
    },
    DuplicateOutgoingMessageId(String),
    DuplicateGroupMessageId(String),
    MissingPendingMessage(String),
    MessageOrderConflict(u64),
    MessageIdConflict(String),
    GroupMessageNumberConflict(u32),
    IncomingOrderTooFarAhead {
        expected: u64,
        received: u64,
        max_gap: u64,
    },
    EmptyGroupMembership,
    GroupMembershipLimitExceeded(usize),
    DuplicateGroupParticipant(String),
    MissingGroupParticipant(String),
    GroupParticipantMismatch(&'static str),
    LocalDeviceMissingFromGroup,
    GroupIdMismatch {
        expected: String,
        received: String,
    },
    GroupEpochMismatch {
        expected: u64,
        received: u64,
    },
    GroupEpochOverflow,
    UnknownSenderKey(String),
    SenderKeyDistributionConflict {
        device_id: String,
        epoch: u64,
    },
    InvalidGroupSignature,
    InvalidGroupKeyMaterial(&'static str),
}

impl fmt::Display for MessagingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => write!(formatter, "core error: {error}"),
            Self::Crypto(error) => write!(formatter, "crypto error: {error}"),
            Self::Transport(error) => write!(formatter, "transport error: {error}"),
            Self::Serialization(error) => write!(formatter, "serialization error: {error}"),
            Self::InvalidHandshakeVersion(version) => {
                write!(
                    formatter,
                    "unsupported secure-session handshake version {version}"
                )
            }
            Self::InvalidEnvelopeVersion(version) => {
                write!(
                    formatter,
                    "unsupported messaging envelope version {version}"
                )
            }
            Self::InvalidSenderKeyDistributionVersion(version) => write!(
                formatter,
                "unsupported sender-key distribution version {version}"
            ),
            Self::InvalidGroupMessageVersion(version) => {
                write!(formatter, "unsupported group message version {version}")
            }
            Self::UnexpectedFrame(label) => {
                write!(formatter, "unexpected transport frame: {label}")
            }
            Self::LocalDeviceIdentityMismatch => formatter.write_str(
                "local device identity keys do not match the configured private identity",
            ),
            Self::RemoteOfferMismatch(label) => {
                write!(formatter, "remote session offer mismatch: {label}")
            }
            Self::RemoteBindingMismatch(label) => {
                write!(formatter, "remote session binding mismatch: {label}")
            }
            Self::TransportBindingMismatch => formatter.write_str(
                "transport certificate binding did not match the secure-session metadata",
            ),
            Self::SessionPeerMismatch => {
                formatter.write_str("messaging engine was used with a different secure session")
            }
            Self::InvalidIdentifier { field, value } => {
                write!(
                    formatter,
                    "{field} '{value}' contains unsupported characters"
                )
            }
            Self::DuplicateOutgoingMessageId(message_id) => {
                write!(formatter, "outgoing message '{message_id}' already exists")
            }
            Self::DuplicateGroupMessageId(message_id) => {
                write!(
                    formatter,
                    "group message '{message_id}' already exists in this epoch"
                )
            }
            Self::MissingPendingMessage(message_id) => {
                write!(formatter, "pending message '{message_id}' was not found")
            }
            Self::MessageOrderConflict(order) => {
                write!(
                    formatter,
                    "conflicting messages were received for delivery order {order}"
                )
            }
            Self::MessageIdConflict(message_id) => {
                write!(
                    formatter,
                    "message id '{message_id}' was reused with different metadata"
                )
            }
            Self::GroupMessageNumberConflict(message_number) => {
                write!(
                    formatter,
                    "group message number {message_number} was reused with different metadata"
                )
            }
            Self::IncomingOrderTooFarAhead {
                expected,
                received,
                max_gap,
            } => write!(
                formatter,
                "incoming delivery order advanced too far: expected={expected}, received={received}, max_gap={max_gap}"
            ),
            Self::EmptyGroupMembership => formatter.write_str("group membership cannot be empty"),
            Self::GroupMembershipLimitExceeded(count) => write!(
                formatter,
                "group membership exceeds the supported limit of 8 participants: {count}"
            ),
            Self::DuplicateGroupParticipant(device_id) => {
                write!(formatter, "group participant '{device_id}' already exists")
            }
            Self::MissingGroupParticipant(device_id) => {
                write!(formatter, "group participant '{device_id}' was not found")
            }
            Self::GroupParticipantMismatch(label) => {
                write!(formatter, "group participant mismatch: {label}")
            }
            Self::LocalDeviceMissingFromGroup => {
                formatter.write_str("local device must be present in the group membership")
            }
            Self::GroupIdMismatch { expected, received } => {
                write!(
                    formatter,
                    "group id mismatch: expected '{expected}', got '{received}'"
                )
            }
            Self::GroupEpochMismatch { expected, received } => write!(
                formatter,
                "group epoch mismatch: expected {expected}, got {received}"
            ),
            Self::GroupEpochOverflow => formatter.write_str("group epoch counter overflowed"),
            Self::UnknownSenderKey(device_id) => {
                write!(
                    formatter,
                    "sender key for device '{device_id}' is not available"
                )
            }
            Self::SenderKeyDistributionConflict { device_id, epoch } => write!(
                formatter,
                "sender-key distribution conflict for device '{device_id}' in epoch {epoch}"
            ),
            Self::InvalidGroupSignature => {
                formatter.write_str("group message signature verification failed")
            }
            Self::InvalidGroupKeyMaterial(label) => {
                write!(formatter, "invalid group key material for {label}")
            }
        }
    }
}

impl Error for MessagingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::Crypto(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::Serialization(_)
            | Self::InvalidHandshakeVersion(_)
            | Self::InvalidEnvelopeVersion(_)
            | Self::InvalidSenderKeyDistributionVersion(_)
            | Self::InvalidGroupMessageVersion(_)
            | Self::UnexpectedFrame(_)
            | Self::LocalDeviceIdentityMismatch
            | Self::RemoteOfferMismatch(_)
            | Self::RemoteBindingMismatch(_)
            | Self::TransportBindingMismatch
            | Self::SessionPeerMismatch
            | Self::InvalidIdentifier { .. }
            | Self::DuplicateOutgoingMessageId(_)
            | Self::DuplicateGroupMessageId(_)
            | Self::MissingPendingMessage(_)
            | Self::MessageOrderConflict(_)
            | Self::MessageIdConflict(_)
            | Self::GroupMessageNumberConflict(_)
            | Self::IncomingOrderTooFarAhead { .. }
            | Self::EmptyGroupMembership
            | Self::GroupMembershipLimitExceeded(_)
            | Self::DuplicateGroupParticipant(_)
            | Self::MissingGroupParticipant(_)
            | Self::GroupParticipantMismatch(_)
            | Self::LocalDeviceMissingFromGroup
            | Self::GroupIdMismatch { .. }
            | Self::GroupEpochMismatch { .. }
            | Self::GroupEpochOverflow
            | Self::UnknownSenderKey(_)
            | Self::SenderKeyDistributionConflict { .. }
            | Self::InvalidGroupSignature
            | Self::InvalidGroupKeyMaterial(_) => None,
        }
    }
}

impl From<CoreError> for MessagingError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

impl From<CryptoError> for MessagingError {
    fn from(error: CryptoError) -> Self {
        Self::Crypto(error)
    }
}

impl From<TransportError> for MessagingError {
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}

impl From<Box<bincode::ErrorKind>> for MessagingError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        Self::Serialization(error.to_string())
    }
}
