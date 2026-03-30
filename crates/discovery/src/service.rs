use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{StreamExt, pin_mut};
use libmdns::{Responder, Service};
use localmessenger_core::DeviceId;
use mdns::{RecordKind, Response};
use tokio::runtime::Handle;
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;

use crate::error::DiscoveryError;
use crate::peer::{DiscoveredPeer, LocalPeerAnnouncement};
use crate::registry::{PeerRegistry, RegistryChange};
use crate::txt::{decode_txt_records, encode_txt_records};

pub const DEFAULT_SERVICE_TYPE: &str = "_localmsg._udp";

#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub service_type: String,
    pub browse_interval: Duration,
    pub stale_after: Duration,
    pub prune_interval: Duration,
    pub event_channel_capacity: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            service_type: DEFAULT_SERVICE_TYPE.to_string(),
            browse_interval: Duration::from_secs(15),
            stale_after: Duration::from_secs(45),
            prune_interval: Duration::from_secs(5),
            event_channel_capacity: 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryEvent {
    PeerAdded(DiscoveredPeer),
    PeerUpdated(DiscoveredPeer),
    PeerExpired(DiscoveredPeer),
}

pub struct DiscoveryService {
    registry: Arc<RwLock<PeerRegistry>>,
    events: broadcast::Sender<DiscoveryEvent>,
    _responder: Responder,
    _service: Service,
    browse_task: JoinHandle<()>,
    prune_task: JoinHandle<()>,
}

impl DiscoveryService {
    pub fn start(
        config: DiscoveryConfig,
        local_peer: LocalPeerAnnouncement,
    ) -> Result<Self, DiscoveryError> {
        validate_service_type(&config.service_type)?;

        let handle =
            Handle::try_current().map_err(|error| DiscoveryError::Responder(error.to_string()))?;
        let responder = Responder::spawn(&handle)
            .map_err(|error| DiscoveryError::Responder(error.to_string()))?;

        let txt_records = encode_txt_records(&local_peer);
        let txt_refs = txt_records.iter().map(String::as_str).collect::<Vec<_>>();
        let service = responder.register(
            &config.service_type,
            &local_peer.instance_name(),
            local_peer.port,
            &txt_refs,
        );

        let registry = Arc::new(RwLock::new(PeerRegistry::new(config.stale_after)));
        let (events, _) = broadcast::channel(config.event_channel_capacity);

        let browse_registry = Arc::clone(&registry);
        let browse_events = events.clone();
        let browse_service_name = browse_service_name(&config.service_type);
        let browse_interval = config.browse_interval;
        let local_device_id = local_peer.device_id.clone();
        let browse_task = tokio::spawn(async move {
            if let Err(error) = browse_loop(
                browse_service_name,
                browse_interval,
                local_device_id,
                browse_registry,
                browse_events,
            )
            .await
            {
                let _ = error;
            }
        });

        let prune_registry = Arc::clone(&registry);
        let prune_events = events.clone();
        let prune_interval = config.prune_interval;
        let prune_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(prune_interval);
            loop {
                interval.tick().await;
                let expired = {
                    let mut registry = prune_registry.write().await;
                    registry.expire_stale_at(Instant::now())
                };
                for change in expired {
                    if let RegistryChange::Expired(peer) = change {
                        let _ = prune_events.send(DiscoveryEvent::PeerExpired(peer));
                    }
                }
            }
        });

        Ok(Self {
            registry,
            events,
            _responder: responder,
            _service: service,
            browse_task,
            prune_task,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryEvent> {
        self.events.subscribe()
    }

    pub async fn snapshot(&self) -> Vec<DiscoveredPeer> {
        self.registry.read().await.snapshot()
    }
}

impl Drop for DiscoveryService {
    fn drop(&mut self) {
        self.browse_task.abort();
        self.prune_task.abort();
    }
}

async fn browse_loop(
    browse_service_name: String,
    browse_interval: Duration,
    local_device_id: DeviceId,
    registry: Arc<RwLock<PeerRegistry>>,
    events: broadcast::Sender<DiscoveryEvent>,
) -> Result<(), DiscoveryError> {
    let stream = mdns::discover::all(&browse_service_name, browse_interval)
        .map_err(|error| DiscoveryError::Browser(error.to_string()))?
        .listen();
    pin_mut!(stream);

    while let Some(result) = stream.next().await {
        let response = match result {
            Ok(response) => response,
            Err(_) => continue,
        };

        let Some(peer) = peer_from_response(&response)? else {
            continue;
        };

        if peer.device_id == local_device_id {
            continue;
        }

        let change = {
            let mut registry = registry.write().await;
            registry.upsert_at(peer, Instant::now())
        };

        match change {
            RegistryChange::Added(peer) => {
                let _ = events.send(DiscoveryEvent::PeerAdded(peer));
            }
            RegistryChange::Updated(peer) => {
                let _ = events.send(DiscoveryEvent::PeerUpdated(peer));
            }
            RegistryChange::Expired(_) | RegistryChange::Unchanged => {}
        }
    }

    Ok(())
}

pub(crate) fn peer_from_response(
    response: &Response,
) -> Result<Option<DiscoveredPeer>, DiscoveryError> {
    let txt_values = response
        .txt_records()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if txt_values.is_empty() {
        return Ok(None);
    }

    let payload = decode_txt_records(txt_values)?;
    let service_instance = service_instance_from_response(response)
        .or_else(|| response.hostname().map(str::to_string))
        .unwrap_or_else(|| payload.device_id.to_string());
    let socket_address = response.socket_address().or_else(|| {
        response
            .ip_addr()
            .map(|ip| std::net::SocketAddr::new(ip, payload.port))
    });
    let hostname = response.hostname().map(str::to_string);

    Ok(Some(payload.into_peer(
        service_instance,
        socket_address,
        hostname,
    )))
}

fn service_instance_from_response(response: &Response) -> Option<String> {
    response.records().find_map(|record| match &record.kind {
        RecordKind::PTR(value) => Some(trim_trailing_dot(value)),
        RecordKind::SRV { .. } => Some(trim_trailing_dot(&record.name)),
        _ => None,
    })
}

fn validate_service_type(service_type: &str) -> Result<(), DiscoveryError> {
    if service_type.trim().is_empty()
        || !service_type.starts_with('_')
        || !service_type.contains("._")
    {
        return Err(DiscoveryError::InvalidServiceType(service_type.to_string()));
    }
    Ok(())
}

fn browse_service_name(service_type: &str) -> String {
    if service_type.ends_with(".local") {
        service_type.to_string()
    } else {
        format!("{service_type}.local")
    }
}

fn trim_trailing_dot(value: &str) -> String {
    value.trim_end_matches('.').to_string()
}
