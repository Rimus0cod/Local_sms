use localmessenger_core::CoreError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum DiscoveryError {
    InvalidTxtRecord(String),
    MissingTxtField(&'static str),
    InvalidCapability(String),
    InvalidServiceType(String),
    InvalidServicePort(u16),
    Browser(String),
    Responder(String),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTxtRecord(value) => write!(formatter, "invalid TXT record '{value}'"),
            Self::MissingTxtField(field) => write!(formatter, "missing TXT field '{field}'"),
            Self::InvalidCapability(value) => write!(formatter, "invalid capability '{value}'"),
            Self::InvalidServiceType(value) => write!(formatter, "invalid service type '{value}'"),
            Self::InvalidServicePort(port) => write!(formatter, "invalid service port {port}"),
            Self::Browser(error) => write!(formatter, "mDNS browser error: {error}"),
            Self::Responder(error) => write!(formatter, "mDNS responder error: {error}"),
        }
    }
}

impl Error for DiscoveryError {}

impl From<CoreError> for DiscoveryError {
    fn from(error: CoreError) -> Self {
        Self::InvalidTxtRecord(error.to_string())
    }
}
