use crate::config::GroupPolicy;
use crate::security::VerificationState;
use std::error::Error;
use std::fmt;
use std::time::SystemTime;

macro_rules! identifier_type {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
                let value = value.into();
                validate_identifier($label, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

identifier_type!(GroupId, "group_id");
identifier_type!(MemberId, "member_id");
identifier_type!(DeviceId, "device_id");
identifier_type!(MessageId, "message_id");

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::EmptyIdentifier(field));
    }

    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        Ok(())
    } else {
        Err(ValidationError::InvalidIdentifier {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_display_name(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError::EmptyDisplayName)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Android,
    Macos,
    Linux,
    Ios,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceState {
    Offline,
    LanOnline,
    P2pOnline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceProfile {
    pub device_id: DeviceId,
    pub device_name: String,
    pub platform: Platform,
    pub verification_state: VerificationState,
}

impl DeviceProfile {
    pub fn new(
        device_id: DeviceId,
        device_name: impl Into<String>,
        platform: Platform,
        verification_state: VerificationState,
    ) -> Result<Self, ValidationError> {
        let device_name = device_name.into();
        validate_display_name(&device_name)?;

        Ok(Self {
            device_id,
            device_name,
            platform,
            verification_state,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberProfile {
    pub member_id: MemberId,
    pub display_name: String,
    pub devices: Vec<DeviceProfile>,
    pub presence: PresenceState,
    pub safety_number: String,
}

impl MemberProfile {
    pub fn new(
        member_id: MemberId,
        display_name: impl Into<String>,
        devices: Vec<DeviceProfile>,
        presence: PresenceState,
        safety_number: impl Into<String>,
    ) -> Result<Self, ValidationError> {
        let display_name = display_name.into();
        validate_display_name(&display_name)?;

        if devices.is_empty() {
            return Err(ValidationError::MissingDevice);
        }

        Ok(Self {
            member_id,
            display_name,
            devices,
            presence,
            safety_number: safety_number.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRoster {
    pub owner_id: MemberId,
    pub members: Vec<MemberProfile>,
}

impl GroupRoster {
    pub fn new(owner: MemberProfile) -> Self {
        Self {
            owner_id: owner.member_id.clone(),
            members: vec![owner],
        }
    }

    pub fn add_member(
        &mut self,
        member: MemberProfile,
        policy: &GroupPolicy,
    ) -> Result<(), RosterError> {
        if self
            .members
            .iter()
            .any(|existing| existing.member_id == member.member_id)
        {
            return Err(RosterError::DuplicateMember(member.member_id.to_string()));
        }

        if self.members.len() >= policy.max_members {
            return Err(RosterError::GroupFull {
                max_members: policy.max_members,
            });
        }

        self.members.push(member);
        Ok(())
    }

    pub fn remove_member(&mut self, member_id: &MemberId) -> Result<bool, RosterError> {
        if member_id == &self.owner_id {
            return Err(RosterError::CannotRemoveOwner);
        }

        let previous_len = self.members.len();
        self.members.retain(|member| &member.member_id != member_id);
        Ok(previous_len != self.members.len())
    }

    pub fn online_members(&self) -> Vec<&MemberProfile> {
        self.members
            .iter()
            .filter(|member| member.presence != PresenceState::Offline)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentKind {
    Image,
    Video,
    File,
    VoiceNote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentMeta {
    pub attachment_id: String,
    pub file_name: String,
    pub kind: AttachmentKind,
    pub size_bytes: u64,
}

impl AttachmentMeta {
    pub fn fits_policy(&self, policy: &GroupPolicy) -> bool {
        self.size_bytes <= u64::from(policy.max_attachment_size_mb) * 1024 * 1024
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageReaction {
    pub emoji: String,
    pub author_id: MemberId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageKind {
    Text,
    Attachment,
    VoiceNote,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub message_id: MessageId,
    pub author_id: MemberId,
    pub kind: MessageKind,
    pub text: String,
    pub reply_to: Option<MessageId>,
    pub attachments: Vec<AttachmentMeta>,
    pub reactions: Vec<MessageReaction>,
    pub created_at: SystemTime,
}

impl ChatMessage {
    pub fn validate_against(&self, policy: &GroupPolicy) -> Result<(), MessageValidationError> {
        let has_text = !self.text.trim().is_empty();
        let has_attachments = !self.attachments.is_empty();

        if !has_text && !has_attachments && self.kind != MessageKind::System {
            return Err(MessageValidationError::EmptyPayload);
        }

        if matches!(self.kind, MessageKind::Text) && !has_text {
            return Err(MessageValidationError::MissingText);
        }

        if self.attachments.len() > 8 {
            return Err(MessageValidationError::TooManyAttachments(
                self.attachments.len(),
            ));
        }

        let limit_bytes = u64::from(policy.max_attachment_size_mb) * 1024 * 1024;
        for attachment in &self.attachments {
            if !attachment.fits_policy(policy) {
                return Err(MessageValidationError::AttachmentTooLarge {
                    file_name: attachment.file_name.clone(),
                    size_bytes: attachment.size_bytes,
                    limit_bytes,
                });
            }
        }

        for reaction in &self.reactions {
            if reaction.emoji.trim().is_empty() {
                return Err(MessageValidationError::EmptyReactionEmoji);
            }
        }

        Ok(())
    }

    pub fn searchable_text(&self) -> String {
        let attachment_names = self
            .attachments
            .iter()
            .map(|attachment| attachment.file_name.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if attachment_names.is_empty() {
            self.text.clone()
        } else if self.text.trim().is_empty() {
            attachment_names
        } else {
            format!("{} {}", self.text, attachment_names)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyIdentifier(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    EmptyDisplayName,
    MissingDevice,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(field) => write!(formatter, "{field} cannot be empty"),
            Self::InvalidIdentifier { field, value } => write!(
                formatter,
                "{field} '{value}' must contain only ASCII letters, digits, '-' or '_'"
            ),
            Self::EmptyDisplayName => formatter.write_str("display name cannot be empty"),
            Self::MissingDevice => formatter.write_str("at least one device must be present"),
        }
    }
}

impl Error for ValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RosterError {
    DuplicateMember(String),
    GroupFull { max_members: usize },
    CannotRemoveOwner,
}

impl fmt::Display for RosterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMember(member_id) => {
                write!(formatter, "member '{member_id}' is already in the roster")
            }
            Self::GroupFull { max_members } => {
                write!(formatter, "group is full, limit is {max_members} members")
            }
            Self::CannotRemoveOwner => formatter.write_str("group owner cannot be removed"),
        }
    }
}

impl Error for RosterError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageValidationError {
    EmptyPayload,
    MissingText,
    TooManyAttachments(usize),
    AttachmentTooLarge {
        file_name: String,
        size_bytes: u64,
        limit_bytes: u64,
    },
    EmptyReactionEmoji,
}

impl fmt::Display for MessageValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPayload => formatter.write_str("message cannot be empty"),
            Self::MissingText => formatter.write_str("text message must include text"),
            Self::TooManyAttachments(count) => {
                write!(formatter, "message contains too many attachments: {count}")
            }
            Self::AttachmentTooLarge {
                file_name,
                size_bytes,
                limit_bytes,
            } => write!(
                formatter,
                "attachment '{file_name}' has size {size_bytes} bytes which exceeds limit {limit_bytes}"
            ),
            Self::EmptyReactionEmoji => formatter.write_str("reaction emoji cannot be empty"),
        }
    }
}

impl Error for MessageValidationError {}
