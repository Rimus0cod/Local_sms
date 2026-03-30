use crate::security::CryptoProfile;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportMode {
    LocalLanMdns,
    DirectP2p,
    BluetoothFallback,
}

impl TransportMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::LocalLanMdns => "LAN + mDNS",
            Self::DirectP2p => "Direct P2P",
            Self::BluetoothFallback => "Bluetooth fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupPolicy {
    pub max_members: usize,
    pub max_attachment_size_mb: u16,
    pub history_export_allowed: bool,
    pub search_enabled: bool,
    pub notifications_enabled: bool,
    pub reactions_enabled: bool,
    pub replies_enabled: bool,
    pub voice_notes_enabled: bool,
    pub calls_enabled: bool,
    pub multi_device_enabled: bool,
    pub disappearing_messages_enabled: bool,
    pub editing_enabled: bool,
}

impl GroupPolicy {
    pub const HARD_MAX_MEMBERS: usize = 8;

    pub fn mvp() -> Self {
        Self {
            max_members: Self::HARD_MAX_MEMBERS,
            max_attachment_size_mb: 100,
            history_export_allowed: true,
            search_enabled: true,
            notifications_enabled: true,
            reactions_enabled: true,
            replies_enabled: true,
            voice_notes_enabled: false,
            calls_enabled: false,
            multi_device_enabled: false,
            disappearing_messages_enabled: false,
            editing_enabled: true,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(2..=Self::HARD_MAX_MEMBERS).contains(&self.max_members) {
            return Err(ConfigError::InvalidMaxMembers(self.max_members));
        }

        if self.max_attachment_size_mb == 0 {
            return Err(ConfigError::InvalidAttachmentLimit(0));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupConfig {
    pub group_id: String,
    pub group_name: String,
    pub languages: Vec<String>,
    pub transport_modes: Vec<TransportMode>,
    pub crypto_profile: CryptoProfile,
    pub policy: GroupPolicy,
}

impl GroupConfig {
    pub fn demo() -> Self {
        Self {
            group_id: "rimus-room".to_string(),
            group_name: "Rimus Local Club".to_string(),
            languages: vec!["ru".to_string(), "en".to_string()],
            transport_modes: vec![TransportMode::LocalLanMdns, TransportMode::BluetoothFallback],
            crypto_profile: CryptoProfile::SignalStyleSenderKeys,
            policy: GroupPolicy::mvp(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.group_id.trim().is_empty() || !is_valid_group_id(&self.group_id) {
            return Err(ConfigError::InvalidGroupId(self.group_id.clone()));
        }

        if self.group_name.trim().is_empty() {
            return Err(ConfigError::EmptyGroupName);
        }

        if self.languages.is_empty() || self.languages.iter().any(|value| value.trim().is_empty()) {
            return Err(ConfigError::MissingLanguage);
        }

        if self.transport_modes.is_empty() {
            return Err(ConfigError::MissingTransport);
        }

        self.policy.validate()
    }
}

fn is_valid_group_id(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-')
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    EmptyGroupName,
    InvalidGroupId(String),
    InvalidMaxMembers(usize),
    InvalidAttachmentLimit(u16),
    MissingTransport,
    MissingLanguage,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGroupName => formatter.write_str("group name cannot be empty"),
            Self::InvalidGroupId(value) => write!(
                formatter,
                "group id '{value}' must contain only lowercase letters, digits or '-'"
            ),
            Self::InvalidMaxMembers(value) => write!(
                formatter,
                "group must allow between 2 and {} members, got {value}",
                GroupPolicy::HARD_MAX_MEMBERS
            ),
            Self::InvalidAttachmentLimit(value) => {
                write!(formatter, "attachment size limit must be positive, got {value}")
            }
            Self::MissingTransport => formatter.write_str("at least one transport mode is required"),
            Self::MissingLanguage => formatter.write_str("at least one UI language is required"),
        }
    }
}

impl Error for ConfigError {}
