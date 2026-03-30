use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use quinn::{IdleTimeout, TransportConfig as QuinnTransportConfig, VarInt};

use crate::error::TransportError;

#[derive(Debug, Clone)]
pub struct TransportEndpointConfig {
    pub bind_addr: SocketAddr,
    pub server_name: String,
    pub alpn_protocol: Vec<u8>,
    pub max_frame_size: usize,
    pub keep_alive_interval: Duration,
    pub idle_timeout: Duration,
    pub max_concurrent_uni_streams: u32,
}

impl TransportEndpointConfig {
    pub fn new(bind_addr: SocketAddr, server_name: impl Into<String>) -> Self {
        Self {
            bind_addr,
            server_name: server_name.into(),
            alpn_protocol: b"localmessenger/transport/v1".to_vec(),
            max_frame_size: 1024 * 1024,
            keep_alive_interval: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(30),
            max_concurrent_uni_streams: 64,
        }
    }

    pub fn recommended(bind_addr: SocketAddr) -> Self {
        Self::new(bind_addr, "localmsg.internal")
    }

    pub fn quinn_transport_config(&self) -> Result<Arc<QuinnTransportConfig>, TransportError> {
        let mut config = QuinnTransportConfig::default();
        config.keep_alive_interval(Some(self.keep_alive_interval));
        config.max_idle_timeout(Some(
            IdleTimeout::try_from(self.idle_timeout)
                .map_err(|error| TransportError::Endpoint(error.to_string()))?,
        ));
        config.max_concurrent_uni_streams(VarInt::from_u32(self.max_concurrent_uni_streams));
        Ok(Arc::new(config))
    }
}

#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    pub max_attempts: usize,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl ReconnectPolicy {
    pub fn new(max_attempts: usize, initial_backoff: Duration, max_backoff: Duration) -> Self {
        Self {
            max_attempts,
            initial_backoff,
            max_backoff,
        }
    }

    pub fn lan_default() -> Self {
        Self::new(8, Duration::from_millis(100), Duration::from_secs(2))
    }

    pub fn backoff_for_attempt(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let factor = 2_u32.saturating_pow((attempt - 1) as u32);
        self.initial_backoff
            .saturating_mul(factor)
            .min(self.max_backoff)
    }
}
