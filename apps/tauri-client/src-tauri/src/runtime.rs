use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use localmessenger_core::{Device, DeviceId, MemberId, MemberProfile};
use localmessenger_crypto::{IdentityKeyMaterial, IdentityKeyPair, LocalPrekeyStore};
use localmessenger_discovery::{DiscoveredPeer, PeerCapability};
use localmessenger_messaging::{
    MessageKind, MessagingEngine, SecureSession, SessionInitiator, SessionResponder,
};
use localmessenger_transport::{
    ReconnectPolicy, TransportEndpoint, TransportEndpointConfig, TransportIdentity,
};
use rand_core::OsRng;

pub struct BootstrapRuntime {
    pub member: MemberProfile,
    pub runtime: DirectChatRuntime,
}

pub struct DirectChatRuntime {
    remote_display_name: String,
    remote_device_id: DeviceId,
    peer: DiscoveredPeer,
    local_session: SecureSession,
    local_engine: MessagingEngine,
    remote_session: SecureSession,
    remote_engine: MessagingEngine,
    reply_script: Vec<String>,
    next_reply_index: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeInboundMessage {
    pub message_id: String,
    pub body: String,
    pub sent_at_unix_ms: i64,
}

#[derive(Debug, Clone)]
pub struct RuntimeSendOutcome {
    pub outbound_acknowledged: bool,
    pub inbound_messages: Vec<RuntimeInboundMessage>,
    pub forward_secrecy_active: bool,
}

impl DirectChatRuntime {
    pub fn remote_display_name(&self) -> &str {
        &self.remote_display_name
    }

    pub fn peer(&self) -> &DiscoveredPeer {
        &self.peer
    }

    pub fn remote_device_id(&self) -> &DeviceId {
        &self.remote_device_id
    }

    pub async fn send_text(
        &mut self,
        conversation_id: &str,
        outbound_message_id: String,
        sent_at_unix_ms: i64,
        body: String,
    ) -> Result<RuntimeSendOutcome, String> {
        self.local_engine
            .send_message(
                &mut self.local_session,
                outbound_message_id,
                conversation_id.to_string(),
                MessageKind::Text,
                sent_at_unix_ms,
                body.into_bytes(),
            )
            .await
            .map_err(|error| error.to_string())?;

        self.remote_engine
            .receive_next(&mut self.remote_session)
            .await
            .map_err(|error| error.to_string())?;

        let ack_outcome = self
            .local_engine
            .receive_next(&mut self.local_session)
            .await
            .map_err(|error| error.to_string())?;

        let mut inbound_messages = Vec::new();
        if let Some(reply_body) = self.next_reply_body() {
            let reply_message_id = format!(
                "{}-{}-reply-{}",
                conversation_id, self.remote_device_id, self.next_reply_index
            );
            let reply_sent_at = sent_at_unix_ms.saturating_add(1);

            self.remote_engine
                .send_message(
                    &mut self.remote_session,
                    reply_message_id.clone(),
                    conversation_id.to_string(),
                    MessageKind::Text,
                    reply_sent_at,
                    reply_body.clone().into_bytes(),
                )
                .await
                .map_err(|error| error.to_string())?;

            let inbound = self
                .local_engine
                .receive_next(&mut self.local_session)
                .await
                .map_err(|error| error.to_string())?;
            self.remote_engine
                .receive_next(&mut self.remote_session)
                .await
                .map_err(|error| error.to_string())?;

            for delivered in inbound.delivered_messages() {
                inbound_messages.push(RuntimeInboundMessage {
                    message_id: delivered.message_id().to_string(),
                    body: String::from_utf8_lossy(delivered.body()).into_owned(),
                    sent_at_unix_ms: delivered.sent_at_unix_ms(),
                });
            }
        }

        Ok(RuntimeSendOutcome {
            outbound_acknowledged: !ack_outcome.acknowledged_message_ids().is_empty()
                && self.local_engine.pending_count() == 0,
            inbound_messages,
            forward_secrecy_active: self
                .local_session
                .forward_secrecy_state()
                .sending_chain_next_message_number()
                .is_some(),
        })
    }

    fn next_reply_body(&mut self) -> Option<String> {
        if self.reply_script.is_empty() {
            return None;
        }

        let index = (self.next_reply_index as usize) % self.reply_script.len();
        let body = self.reply_script[index].clone();
        self.next_reply_index = self.next_reply_index.saturating_add(1);
        Some(body)
    }
}

pub async fn bootstrap_direct_chat_runtime(
    local_device: &Device,
    local_identity_material: &IdentityKeyMaterial,
    remote_member_id: &str,
    remote_display_name: &str,
    remote_device_id: &str,
    remote_device_name: &str,
    prekey_seed: u32,
    capabilities: Vec<PeerCapability>,
    reply_script: Vec<&str>,
) -> Result<BootstrapRuntime, String> {
    let mut rng = OsRng;
    let remote_member_id_value =
        MemberId::new(remote_member_id).map_err(|error| error.to_string())?;
    let remote_device_id_value =
        DeviceId::new(remote_device_id).map_err(|error| error.to_string())?;
    let remote_identity = IdentityKeyPair::generate(&mut rng);
    let remote_device = Device::from_identity_keypair(
        remote_device_id_value.clone(),
        remote_member_id_value.clone(),
        remote_device_name,
        &remote_identity,
    )
    .map_err(|error| error.to_string())?;

    let mut remote_member = MemberProfile::new(remote_member_id_value, remote_display_name)
        .map_err(|error| error.to_string())?;
    remote_member
        .add_device(remote_device.clone())
        .map_err(|error| error.to_string())?;

    let server_config =
        TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
    let server_identity = TransportIdentity::generate(server_config.server_name.clone())
        .map_err(|error| error.to_string())?;
    let server = TransportEndpoint::bind(server_config, server_identity.clone())
        .map_err(|error| error.to_string())?;
    let server_addr = server.local_addr().map_err(|error| error.to_string())?;

    let client_config =
        TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
    let client_identity = TransportIdentity::generate(client_config.server_name.clone())
        .map_err(|error| error.to_string())?;
    let client = TransportEndpoint::bind(client_config, client_identity)
        .map_err(|error| error.to_string())?;

    let remote_prekeys = LocalPrekeyStore::generate(
        &mut rng,
        &remote_identity,
        prekey_seed,
        4,
        prekey_seed * 100,
    );
    let mut responder = SessionResponder::new(
        remote_device.clone(),
        remote_identity,
        remote_prekeys,
        &server_identity.certificate_der,
    )
    .map_err(|error| error.to_string())?;
    let offer = responder
        .remote_session_offer()
        .map_err(|error| error.to_string())?;

    let accept_task = tokio::spawn(async move {
        let connection = server.accept().await.map_err(|error| error.to_string())?;
        responder
            .accept(connection)
            .await
            .map_err(|error| error.to_string())
    });

    let connection = client
        .connect(
            server_addr,
            &server_identity.certificate_der,
            &ReconnectPolicy::lan_default(),
        )
        .await
        .map_err(|error| error.to_string())?;
    let initiator = SessionInitiator::new(
        local_device.clone(),
        IdentityKeyPair::from_material(local_identity_material),
    )
    .map_err(|error| error.to_string())?;
    let local_session = initiator
        .establish(connection, &offer, &server_identity.certificate_der)
        .await
        .map_err(|error| error.to_string())?;
    let remote_session = accept_task.await.map_err(|error| error.to_string())??;

    let peer = DiscoveredPeer {
        service_instance: format!("{remote_device_name}-{remote_device_id}.runtime.local"),
        member_id: remote_member.member_id().clone(),
        device_id: remote_device_id_value.clone(),
        device_name: remote_device_name.to_string(),
        port: server_addr.port(),
        socket_address: Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            server_addr.port(),
        )),
        hostname: Some("runtime.local".to_string()),
        capabilities,
    };

    Ok(BootstrapRuntime {
        member: remote_member,
        runtime: DirectChatRuntime {
            remote_display_name: remote_display_name.to_string(),
            remote_device_id: remote_device_id_value,
            peer,
            local_engine: MessagingEngine::from_session(&local_session),
            local_session,
            remote_engine: MessagingEngine::from_session(&remote_session),
            remote_session,
            reply_script: reply_script.into_iter().map(str::to_string).collect(),
            next_reply_index: 0,
        },
    })
}
