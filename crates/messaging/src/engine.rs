use std::collections::{BTreeMap, BTreeSet};

use localmessenger_core::{Device, DeviceId, MemberId};
use serde::{Deserialize, Serialize};

use crate::envelope::{
    MESSAGING_ENVELOPE_VERSION, MessageKind, MessagingEnvelope, MessagingEnvelopeBody, WireAck,
    WireMessage,
};
use crate::{MessagingError, SecureSession};

const MAX_INCOMING_ORDER_GAP: u64 = 1024;
const MAX_TRACKED_INCOMING_ORDERS: u64 = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutgoingMessage {
    message_id: String,
    conversation_id: String,
    delivery_order: u64,
    sent_at_unix_ms: i64,
    kind: MessageKind,
    body: Vec<u8>,
    attempt_count: u32,
}

impl OutgoingMessage {
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    pub fn delivery_order(&self) -> u64 {
        self.delivery_order
    }

    pub fn sent_at_unix_ms(&self) -> i64 {
        self.sent_at_unix_ms
    }

    pub fn kind(&self) -> MessageKind {
        self.kind
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveredMessage {
    message_id: String,
    conversation_id: String,
    delivery_order: u64,
    sent_at_unix_ms: i64,
    sender_member_id: MemberId,
    sender_device_id: DeviceId,
    kind: MessageKind,
    body: Vec<u8>,
}

impl DeliveredMessage {
    pub fn message_id(&self) -> &str {
        &self.message_id
    }

    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    pub fn delivery_order(&self) -> u64 {
        self.delivery_order
    }

    pub fn sent_at_unix_ms(&self) -> i64 {
        self.sent_at_unix_ms
    }

    pub fn sender_member_id(&self) -> &MemberId {
        &self.sender_member_id
    }

    pub fn sender_device_id(&self) -> &DeviceId {
        &self.sender_device_id
    }

    pub fn kind(&self) -> MessageKind {
        self.kind
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReceiveOutcome {
    delivered_messages: Vec<DeliveredMessage>,
    acknowledged_message_ids: Vec<String>,
}

impl ReceiveOutcome {
    pub fn delivered_messages(&self) -> &[DeliveredMessage] {
        &self.delivered_messages
    }

    pub fn acknowledged_message_ids(&self) -> &[String] {
        &self.acknowledged_message_ids
    }

    pub fn is_idle(&self) -> bool {
        self.delivered_messages.is_empty() && self.acknowledged_message_ids.is_empty()
    }
}

pub struct MessagingEngine {
    local_device: Device,
    remote_device: Device,
    next_outgoing_order: u64,
    next_expected_incoming_order: u64,
    max_incoming_order_gap: u64,
    pending_by_order: BTreeMap<u64, OutgoingMessage>,
    pending_index: BTreeMap<String, u64>,
    outgoing_message_ids: BTreeSet<String>,
    incoming_message_orders: BTreeMap<String, u64>,
    incoming_order_index: BTreeMap<u64, String>,
    buffered_incoming: BTreeMap<u64, WireMessage>,
}

/// A portable snapshot of the pending outbound queue that can be serialised to
/// SQLite and restored on the next startup so unacknowledged messages survive
/// application restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQueueSnapshot {
    /// The next `delivery_order` counter that will be assigned to a freshly
    /// queued message.  Must be restored so the ordering stays contiguous.
    pub next_outgoing_order: u64,
    /// All messages that have been sent but not yet acknowledged at the time
    /// the snapshot was taken.
    pub pending_messages: Vec<OutgoingMessage>,
}

impl MessagingEngine {
    pub fn new(local_device: Device, remote_device: Device) -> Self {
        Self {
            local_device,
            remote_device,
            next_outgoing_order: 0,
            next_expected_incoming_order: 0,
            max_incoming_order_gap: MAX_INCOMING_ORDER_GAP,
            pending_by_order: BTreeMap::new(),
            pending_index: BTreeMap::new(),
            outgoing_message_ids: BTreeSet::new(),
            incoming_message_orders: BTreeMap::new(),
            incoming_order_index: BTreeMap::new(),
            buffered_incoming: BTreeMap::new(),
        }
    }

    pub fn from_session(session: &SecureSession) -> Self {
        Self::new(
            session.local_device().clone(),
            session.remote_device().clone(),
        )
    }

    pub fn local_device(&self) -> &Device {
        &self.local_device
    }

    pub fn remote_device(&self) -> &Device {
        &self.remote_device
    }

    pub fn pending_count(&self) -> usize {
        self.pending_by_order.len()
    }

    pub fn pending_messages(&self) -> Vec<OutgoingMessage> {
        self.pending_by_order.values().cloned().collect()
    }

    pub fn is_pending(&self, message_id: &str) -> bool {
        self.pending_index.contains_key(message_id)
    }

    pub fn next_expected_incoming_order(&self) -> u64 {
        self.next_expected_incoming_order
    }

    pub fn queue_message(
        &mut self,
        message_id: impl Into<String>,
        conversation_id: impl Into<String>,
        kind: MessageKind,
        sent_at_unix_ms: i64,
        body: Vec<u8>,
    ) -> Result<OutgoingMessage, MessagingError> {
        let message_id = message_id.into();
        let conversation_id = conversation_id.into();
        validate_identifier("message_id", &message_id)?;
        validate_identifier("conversation_id", &conversation_id)?;

        if !self.outgoing_message_ids.insert(message_id.clone()) {
            return Err(MessagingError::DuplicateOutgoingMessageId(message_id));
        }

        let message = OutgoingMessage {
            message_id: message_id.clone(),
            conversation_id,
            delivery_order: self.next_outgoing_order,
            sent_at_unix_ms,
            kind,
            body,
            attempt_count: 0,
        };
        self.next_outgoing_order += 1;
        self.pending_index
            .insert(message_id, message.delivery_order);
        self.pending_by_order
            .insert(message.delivery_order, message.clone());

        Ok(message)
    }

    pub async fn send_message(
        &mut self,
        session: &mut SecureSession,
        message_id: impl Into<String>,
        conversation_id: impl Into<String>,
        kind: MessageKind,
        sent_at_unix_ms: i64,
        body: Vec<u8>,
    ) -> Result<OutgoingMessage, MessagingError> {
        let queued =
            self.queue_message(message_id, conversation_id, kind, sent_at_unix_ms, body)?;
        self.retry_message(session, queued.message_id()).await
    }

    pub async fn retry_message(
        &mut self,
        session: &mut SecureSession,
        message_id: &str,
    ) -> Result<OutgoingMessage, MessagingError> {
        self.ensure_session_matches(session)?;
        let order = *self
            .pending_index
            .get(message_id)
            .ok_or_else(|| MessagingError::MissingPendingMessage(message_id.to_string()))?;
        self.send_pending_order(session, order).await
    }

    pub async fn flush_pending(
        &mut self,
        session: &mut SecureSession,
    ) -> Result<usize, MessagingError> {
        self.ensure_session_matches(session)?;
        let orders: Vec<u64> = self.pending_by_order.keys().copied().collect();
        for order in &orders {
            self.send_pending_order(session, *order).await?;
        }
        Ok(orders.len())
    }

    pub async fn receive_next(
        &mut self,
        session: &mut SecureSession,
    ) -> Result<ReceiveOutcome, MessagingError> {
        self.ensure_session_matches(session)?;

        let bytes = session.receive_encrypted().await?;
        let envelope: MessagingEnvelope = bincode::deserialize(&bytes)?;
        if envelope.version != MESSAGING_ENVELOPE_VERSION {
            return Err(MessagingError::InvalidEnvelopeVersion(envelope.version));
        }

        match envelope.body {
            MessagingEnvelopeBody::Ack(ack) => Ok(self.handle_ack(ack)),
            MessagingEnvelopeBody::Message(message) => {
                self.handle_incoming_message(session, message).await
            }
        }
    }

    async fn send_pending_order(
        &mut self,
        session: &mut SecureSession,
        order: u64,
    ) -> Result<OutgoingMessage, MessagingError> {
        let envelope_bytes = {
            let pending = self
                .pending_by_order
                .get_mut(&order)
                .ok_or_else(|| MessagingError::MissingPendingMessage(order.to_string()))?;
            pending.attempt_count += 1;
            bincode::serialize(&MessagingEnvelope {
                version: MESSAGING_ENVELOPE_VERSION,
                body: MessagingEnvelopeBody::Message(WireMessage {
                    message_id: pending.message_id.clone(),
                    conversation_id: pending.conversation_id.clone(),
                    delivery_order: pending.delivery_order,
                    sent_at_unix_ms: pending.sent_at_unix_ms,
                    kind: pending.kind,
                    body: pending.body.clone(),
                }),
            })?
        };

        session.send_encrypted(&envelope_bytes).await?;
        self.pending_by_order
            .get(&order)
            .cloned()
            .ok_or_else(|| MessagingError::MissingPendingMessage(order.to_string()))
    }

    fn handle_ack(&mut self, ack: WireAck) -> ReceiveOutcome {
        let mut outcome = ReceiveOutcome::default();
        let Some(order) = self.pending_index.remove(&ack.message_id) else {
            return outcome;
        };

        if let Some(pending) = self.pending_by_order.remove(&order) {
            if pending.delivery_order == ack.delivery_order {
                outcome.acknowledged_message_ids.push(ack.message_id);
            } else {
                self.pending_by_order.insert(order, pending);
                self.pending_index.insert(ack.message_id, order);
            }
        }

        outcome
    }

    async fn handle_incoming_message(
        &mut self,
        session: &mut SecureSession,
        message: WireMessage,
    ) -> Result<ReceiveOutcome, MessagingError> {
        validate_identifier("message_id", &message.message_id)?;
        validate_identifier("conversation_id", &message.conversation_id)?;

        if message.delivery_order
            > self
                .next_expected_incoming_order
                .saturating_add(self.max_incoming_order_gap)
        {
            return Err(MessagingError::IncomingOrderTooFarAhead {
                expected: self.next_expected_incoming_order,
                received: message.delivery_order,
                max_gap: self.max_incoming_order_gap,
            });
        }

        if let Some(existing_order) = self.incoming_message_orders.get(&message.message_id) {
            if *existing_order != message.delivery_order {
                return Err(MessagingError::MessageIdConflict(message.message_id));
            }
            self.send_ack(session, &message.message_id, message.delivery_order)
                .await?;
            return Ok(ReceiveOutcome::default());
        }

        if let Some(existing_message_id) = self.incoming_order_index.get(&message.delivery_order) {
            if existing_message_id != &message.message_id {
                return Err(MessagingError::MessageOrderConflict(message.delivery_order));
            }
            self.send_ack(session, &message.message_id, message.delivery_order)
                .await?;
            return Ok(ReceiveOutcome::default());
        }

        self.send_ack(session, &message.message_id, message.delivery_order)
            .await?;

        self.incoming_message_orders
            .insert(message.message_id.clone(), message.delivery_order);
        self.incoming_order_index
            .insert(message.delivery_order, message.message_id.clone());

        if message.delivery_order < self.next_expected_incoming_order {
            return Ok(ReceiveOutcome::default());
        }

        if message.delivery_order > self.next_expected_incoming_order {
            self.buffered_incoming
                .insert(message.delivery_order, message);
            return Ok(ReceiveOutcome::default());
        }

        let mut outcome = ReceiveOutcome::default();
        self.deliver_message(message, &mut outcome.delivered_messages);

        while let Some(next) = self
            .buffered_incoming
            .remove(&self.next_expected_incoming_order)
        {
            self.deliver_message(next, &mut outcome.delivered_messages);
        }

        Ok(outcome)
    }

    async fn send_ack(
        &self,
        session: &mut SecureSession,
        message_id: &str,
        delivery_order: u64,
    ) -> Result<(), MessagingError> {
        let envelope = MessagingEnvelope {
            version: MESSAGING_ENVELOPE_VERSION,
            body: MessagingEnvelopeBody::Ack(WireAck {
                message_id: message_id.to_string(),
                delivery_order,
            }),
        };
        session
            .send_encrypted(&bincode::serialize(&envelope)?)
            .await
    }

    fn deliver_message(
        &mut self,
        message: WireMessage,
        delivered_messages: &mut Vec<DeliveredMessage>,
    ) {
        delivered_messages.push(DeliveredMessage {
            message_id: message.message_id,
            conversation_id: message.conversation_id,
            delivery_order: message.delivery_order,
            sent_at_unix_ms: message.sent_at_unix_ms,
            sender_member_id: self.remote_device.owner_member_id().clone(),
            sender_device_id: self.remote_device.device_id().clone(),
            kind: message.kind,
            body: message.body,
        });
        self.next_expected_incoming_order += 1;
        self.prune_incoming_history();
    }

    fn prune_incoming_history(&mut self) {
        let minimum_order = self
            .next_expected_incoming_order
            .saturating_sub(MAX_TRACKED_INCOMING_ORDERS);

        while let Some((&oldest_order, oldest_message_id)) =
            self.incoming_order_index.first_key_value()
        {
            if oldest_order >= minimum_order {
                break;
            }

            let oldest_message_id = oldest_message_id.clone();
            self.incoming_order_index.remove(&oldest_order);
            self.incoming_message_orders.remove(&oldest_message_id);
        }
    }

    /// Exports the current in-memory pending queue as a portable snapshot that
    /// can be persisted to SQLite and later fed back to
    /// [`MessagingEngine::restore_pending_queue`].
    pub fn export_pending_queue(&self) -> PendingQueueSnapshot {
        PendingQueueSnapshot {
            next_outgoing_order: self.next_outgoing_order,
            pending_messages: self.pending_by_order.values().cloned().collect(),
        }
    }

    /// Restores a previously exported snapshot into a freshly constructed
    /// engine.  Fails if the snapshot contains message IDs that are already
    /// tracked or if identifiers are invalid.
    pub fn restore_pending_queue(
        &mut self,
        snapshot: PendingQueueSnapshot,
    ) -> Result<(), MessagingError> {
        for message in snapshot.pending_messages {
            validate_identifier("message_id", &message.message_id)?;
            validate_identifier("conversation_id", &message.conversation_id)?;
            if !self.outgoing_message_ids.insert(message.message_id.clone()) {
                return Err(MessagingError::DuplicateOutgoingMessageId(
                    message.message_id.clone(),
                ));
            }
            self.pending_index
                .insert(message.message_id.clone(), message.delivery_order);
            self.pending_by_order
                .insert(message.delivery_order, message);
        }
        // Advance the order counter so new messages don't collide.
        if snapshot.next_outgoing_order > self.next_outgoing_order {
            self.next_outgoing_order = snapshot.next_outgoing_order;
        }
        Ok(())
    }

    fn ensure_session_matches(&self, session: &SecureSession) -> Result<(), MessagingError> {
        if session.local_device().device_id() != self.local_device.device_id()
            || session.local_device().owner_member_id() != self.local_device.owner_member_id()
            || session.remote_device().device_id() != self.remote_device.device_id()
            || session.remote_device().owner_member_id() != self.remote_device.owner_member_id()
        {
            return Err(MessagingError::SessionPeerMismatch);
        }
        Ok(())
    }
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), MessagingError> {
    if value.trim().is_empty() {
        return Err(MessagingError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }

    if value.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || character == '-'
            || character == '_'
            || character == ':'
    }) {
        Ok(())
    } else {
        Err(MessagingError::InvalidIdentifier {
            field,
            value: value.to_string(),
        })
    }
}
