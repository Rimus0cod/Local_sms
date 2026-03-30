use serde::{Deserialize, Serialize};

pub(crate) const MESSAGING_ENVELOPE_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    Text,
    Attachment,
    VoiceNote,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WireMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub delivery_order: u64,
    pub sent_at_unix_ms: i64,
    pub kind: MessageKind,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WireAck {
    pub message_id: String,
    pub delivery_order: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MessagingEnvelopeBody {
    Message(WireMessage),
    Ack(WireAck),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MessagingEnvelope {
    pub version: u8,
    pub body: MessagingEnvelopeBody,
}
