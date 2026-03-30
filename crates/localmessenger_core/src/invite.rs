use crate::domain::GroupId;
use std::error::Error;
use std::fmt;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteTransport {
    QrCode,
    TimeLimitedLink,
    ManualCode,
}

impl InviteTransport {
    pub fn label(&self) -> &'static str {
        match self {
            Self::QrCode => "QR code",
            Self::TimeLimitedLink => "time-limited link",
            Self::ManualCode => "manual code",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteToken {
    pub group_id: GroupId,
    pub code: String,
    pub issued_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub max_uses: u8,
    pub used_count: u8,
    pub transport: InviteTransport,
}

impl InviteToken {
    pub fn ephemeral(
        group_id: GroupId,
        code: impl Into<String>,
        ttl: Duration,
        max_uses: u8,
        transport: InviteTransport,
    ) -> Result<Self, InviteError> {
        let code = code.into();
        validate_code(&code)?;

        let issued_at = SystemTime::now();
        Ok(Self {
            group_id,
            code,
            issued_at,
            expires_at: Some(issued_at + ttl),
            max_uses,
            used_count: 0,
            transport,
        })
    }

    pub fn manual(group_id: GroupId, code: impl Into<String>) -> Result<Self, InviteError> {
        let code = code.into();
        validate_code(&code)?;

        Ok(Self {
            group_id,
            code,
            issued_at: SystemTime::now(),
            expires_at: None,
            max_uses: 1,
            used_count: 0,
            transport: InviteTransport::ManualCode,
        })
    }

    pub fn is_valid_at(&self, now: SystemTime) -> bool {
        let not_expired = self.expires_at.is_none_or(|expires_at| now <= expires_at);
        let under_limit = self.used_count < self.max_uses;
        not_expired && under_limit
    }

    pub fn consume(&mut self, now: SystemTime) -> Result<(), InviteError> {
        if self.max_uses == 0 {
            return Err(InviteError::ZeroUseInvite);
        }

        if let Some(expires_at) = self.expires_at {
            if now > expires_at {
                return Err(InviteError::Expired);
            }
        }

        if self.used_count >= self.max_uses {
            return Err(InviteError::UsageLimitReached);
        }

        self.used_count += 1;
        Ok(())
    }
}

fn validate_code(value: &str) -> Result<(), InviteError> {
    if value.trim().is_empty() {
        return Err(InviteError::EmptyCode);
    }

    if value
        .chars()
        .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-')
    {
        Ok(())
    } else {
        Err(InviteError::InvalidCode(value.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteError {
    EmptyCode,
    InvalidCode(String),
    ZeroUseInvite,
    Expired,
    UsageLimitReached,
}

impl fmt::Display for InviteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCode => formatter.write_str("invite code cannot be empty"),
            Self::InvalidCode(value) => {
                write!(formatter, "invite code '{value}' must use only A-Z, 0-9 or '-'")
            }
            Self::ZeroUseInvite => formatter.write_str("invite must allow at least one use"),
            Self::Expired => formatter.write_str("invite has expired"),
            Self::UsageLimitReached => formatter.write_str("invite usage limit reached"),
        }
    }
}

impl Error for InviteError {}
